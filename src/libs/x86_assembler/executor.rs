//! Executor for GCC and objdump.
//!
//! This module handles the low-level execution of GCC (for assembly)
//! and objdump (for disassembly). These functions are intentionally
//! private to this crate to ensure callers must go through the
//! validated public API in mod.rs.

use std::fs;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tempfile::TempDir;

use super::filter::SafeAsm;
use super::parser::{parse_objdump_output, AssemblyResult};

/// Timeout for GCC/objdump processes (30 seconds).
/// This is a defense-in-depth measure; with input filtering, these should complete in fractions of a second.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

/// Architecture for assembly/disassembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X64,
}

impl Arch {
    /// Get the GCC architecture flag.
    fn gcc_flag(&self) -> &'static str {
        match self {
            Arch::X86 => "-m32",
            Arch::X64 => "-m64",
        }
    }

    /// Get the objdump architecture for binary disassembly.
    fn objdump_arch(&self) -> &'static str {
        match self {
            Arch::X86 => "i386",
            Arch::X64 => "i386:x86-64",
        }
    }
}

/// Error type for assembly/disassembly operations.
#[derive(Debug)]
pub enum AssemblerError {
    /// Input exceeded the maximum size limit.
    InputTooLarge,
    /// Input contained unsafe directives.
    UnsafeCode,
    /// GCC or objdump failed with an error message.
    AssemblyFailure(String),
    /// Process timed out.
    Timeout,
    /// Internal error (temp file creation, etc.)
    InternalError(String),
}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssemblerError::InputTooLarge => write!(
                f,
                "Sorry, your input is too large! Maximum input size is 10KB."
            ),
            AssemblerError::UnsafeCode => write!(
                f,
                "{}",
                super::filter::unsafe_directives_message()
            ),
            AssemblerError::AssemblyFailure(msg) => write!(f, "{}", msg),
            AssemblerError::Timeout => write!(f, "Assembly timed out. Please try simpler input."),
            AssemblerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Assemble code using GCC.
///
/// This function is `pub(super)` because it executes external commands.
/// It takes a [`SafeAsm`] which can only be obtained from
/// [`check_code_safety`](super::filter::check_code_safety), ensuring
/// input has been validated at the type level.
///
/// # Arguments
/// * `code` - The validated assembly code
/// * `arch` - Target architecture
///
/// # Returns
/// The assembled result or an error.
pub(super) async fn assemble_unsafe(code: &SafeAsm<'_>, arch: Arch) -> Result<AssemblyResult, AssemblerError> {
    // Create a temporary directory for our files.
    // Using TempDir ensures cleanup even on error.
    let temp_dir = TempDir::new()
        .map_err(|e| AssemblerError::InternalError(format!("Failed to create temp dir: {}", e)))?;

    let source_path = temp_dir.path().join("code.s");
    let obj_path = temp_dir.path().join("code.o");

    // Prepare the assembly source.
    // - .intel_syntax noprefix: Use Intel syntax without register prefixes
    // - _main: label: Required entry point for objdump to find the code
    // SECURITY: Two independent defenses prevent the C preprocessor from running
    // on user code (which would allow #include to read arbitrary files, etc.):
    // 1. Lowercase .s extension (vs .S which enables preprocessing)
    // 2. -x assembler flag explicitly overrides extension-based language detection
    let asm_source = format!(".intel_syntax noprefix\n_main:\n{}\n", code.as_str());

    fs::write(&source_path, &asm_source)
        .map_err(|e| AssemblerError::InternalError(format!("Failed to write source: {}", e)))?;

    // Assemble with GCC (with timeout)
    let gcc_result = timeout(
        PROCESS_TIMEOUT,
        Command::new("gcc")
            .arg(arch.gcc_flag())
            .args(["-x", "assembler"])
            .arg("-c")
            .arg(&source_path)
            .arg("-o")
            .arg(&obj_path)
            .kill_on_drop(true)
            .output(),
    )
    .await;

    let gcc_output = match gcc_result {
        Err(_) => return Err(AssemblerError::Timeout),
        Ok(result) => result
            .map_err(|e| AssemblerError::InternalError(format!("Failed to run gcc: {}", e)))?,
    };

    if !gcc_output.status.success() {
        // Clean up the error message to remove temp file paths
        let stderr = String::from_utf8_lossy(&gcc_output.stderr);
        let cleaned = clean_error_message(&stderr, temp_dir.path().to_str().unwrap_or(""));
        return Err(AssemblerError::AssemblyFailure(cleaned));
    }

    // Disassemble with objdump (with timeout)
    let objdump_result = timeout(
        PROCESS_TIMEOUT,
        Command::new("objdump")
            .arg("-z")           // Show zero bytes
            .arg("-M")
            .arg("intel")        // Intel syntax
            .arg("-d")           // Disassemble
            .arg(&obj_path)
            .kill_on_drop(true)
            .output(),
    )
    .await;

    let objdump_output = match objdump_result {
        Err(_) => return Err(AssemblerError::Timeout),
        Ok(result) => result
            .map_err(|e| AssemblerError::InternalError(format!("Failed to run objdump: {}", e)))?,
    };

    if !objdump_output.status.success() {
        return Err(AssemblerError::AssemblyFailure("Disassembly failed".to_string()));
    }

    let output_str = String::from_utf8_lossy(&objdump_output.stdout);
    parse_objdump_output(&output_str, false)
        .map_err(|e| AssemblerError::AssemblyFailure(e))
}

/// Disassemble binary data using objdump.
///
/// # Safety
///
/// This function is `pub(super)` because it executes external commands.
/// Input is binary data (from hex parsing), so no code validation is needed,
/// but access is still restricted to the module's public API.
/// TODO: this is not actually "unsafe", or at least shouldn't be
///
/// # Arguments
/// * `binary` - The raw binary data to disassemble
/// * `arch` - Target architecture
///
/// # Returns
/// The disassembly result or an error.
pub(super) async fn disassemble_unsafe(binary: &[u8], arch: Arch) -> Result<AssemblyResult, AssemblerError> {
    if binary.is_empty() {
        return Err(AssemblerError::AssemblyFailure("No data to disassemble".to_string()));
    }

    // Create a temporary directory for our files
    let temp_dir = TempDir::new()
        .map_err(|e| AssemblerError::InternalError(format!("Failed to create temp dir: {}", e)))?;

    let binary_path = temp_dir.path().join("code.bin");

    fs::write(&binary_path, binary)
        .map_err(|e| AssemblerError::InternalError(format!("Failed to write binary: {}", e)))?;

    // Disassemble with objdump in binary mode (with timeout)
    let objdump_result = timeout(
        PROCESS_TIMEOUT,
        Command::new("objdump")
            .arg("-z")                    // Show zero bytes
            .arg("-b")
            .arg("binary")                // Input is raw binary
            .arg("-m")
            .arg(arch.objdump_arch())     // Target architecture
            .arg("-M")
            .arg("intel")                 // Intel syntax
            .arg("-D")                    // Disassemble all sections
            .arg(&binary_path)
            .kill_on_drop(true)
            .output(),
    )
    .await;

    let objdump_output = match objdump_result {
        Err(_) => return Err(AssemblerError::Timeout),
        Ok(result) => result
            .map_err(|e| AssemblerError::InternalError(format!("Failed to run objdump: {}", e)))?,
    };

    if !objdump_output.status.success() {
        return Err(AssemblerError::AssemblyFailure("Disassembly failed".to_string()));
    }

    let output_str = String::from_utf8_lossy(&objdump_output.stdout);
    parse_objdump_output(&output_str, true)
        .map_err(|e| AssemblerError::AssemblyFailure(e))
}

/// Clean error messages by removing temp file paths.
///
/// Transforms messages like "/tmp/xyz123/code.s:3: error" to "3: error"
fn clean_error_message(message: &str, temp_path: &str) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    let mut cleaned = message.to_string();

    // Remove the temp directory path and filename
    if !temp_path.is_empty() {
        let source_prefix = format!("{}/code.s:", temp_path);
        cleaned = cleaned.replace(&source_prefix, "");
    }

    // Also try to match generic patterns like /tmp/.../code.s:
    static TEMP_PATH_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/[^\s:]+/code\.s:(\d+:|\s*)").unwrap());
    cleaned = TEMP_PATH_REGEX.replace_all(&cleaned, "").to_string();

    // Remove "Assembler messages:" prefix
    cleaned = cleaned.replace("Assembler messages:\n", "");

    cleaned.trim().to_string()
}

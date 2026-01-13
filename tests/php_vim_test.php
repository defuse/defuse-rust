<?php
// Test script to generate VimHighlight output for comparison with Rust

// Include the VimHighlight class
require_once('/mnt/e/Home/work/defuse-rewrite/defuse.ca/src/libs/VimHighlight.php');

// Test case 1: Simple Ruby code without line numbers
function test_simple_ruby() {
    $hl = new VimHighlight();
    $hl->caching = false;  // Disable caching for test
    $hl->color_scheme = "dw_cyan";
    $hl->show_lines = false;
    $hl->use_css = true;
    $hl->file_type = "ruby";
    $hl->setVimCommand("vim");

    $text = "puts 'hello'";
    $result = $hl->processText($text, true);

    echo "=== Test 1: Simple Ruby ===\n";
    echo $result;
    echo "\n\n";
}

// Test case 2: Ruby with line numbers
function test_ruby_with_lines() {
    $hl = new VimHighlight();
    $hl->caching = false;
    $hl->color_scheme = "dw_cyan";
    $hl->show_lines = true;
    $hl->use_css = true;
    $hl->file_type = "ruby";
    $hl->setVimCommand("vim");

    $text = "x = 1 + 2";
    $result = $hl->processText($text, true);

    echo "=== Test 2: Ruby with line numbers ===\n";
    echo $result;
    echo "\n\n";
}

// Test case 3: Multi-line Ruby
function test_multiline_ruby() {
    $hl = new VimHighlight();
    $hl->caching = false;
    $hl->color_scheme = "dw_cyan";
    $hl->show_lines = false;
    $hl->use_css = true;
    $hl->file_type = "ruby";
    $hl->setVimCommand("vim");

    $text = "def hello\n  puts 'Hello, World!'\nend";
    $result = $hl->processText($text, true);

    echo "=== Test 3: Multi-line Ruby ===\n";
    echo $result;
    echo "\n\n";
}

// Test case 4: Text (no highlighting)
function test_plain_text() {
    $hl = new VimHighlight();
    $hl->caching = false;
    $hl->color_scheme = "dw_cyan";
    $hl->show_lines = false;
    $hl->use_css = true;
    $hl->file_type = "text";
    $hl->setVimCommand("vim");

    $text = "This is plain text\nWith multiple lines";
    $result = $hl->processText($text, true);

    echo "=== Test 4: Plain text ===\n";
    echo $result;
    echo "\n\n";
}

// Test case 5: HTML entities in code
function test_html_entities() {
    $hl = new VimHighlight();
    $hl->caching = false;
    $hl->color_scheme = "dw_cyan";
    $hl->show_lines = false;
    $hl->use_css = true;
    $hl->file_type = "ruby";
    $hl->setVimCommand("vim");

    $text = "x = '<html>' && y > 0";
    $result = $hl->processText($text, true);

    echo "=== Test 5: HTML entities ===\n";
    echo $result;
    echo "\n\n";
}

// Run all tests
test_simple_ruby();
test_ruby_with_lines();
test_multiline_ruby();
test_plain_text();
test_html_entities();

echo "=== PHP Tests Complete ===\n";

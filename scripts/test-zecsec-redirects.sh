#!/usr/bin/env bash
#
# Tests that all old zecsec.com URLs redirect to the correct defuse.ca destinations.
# Expects the Caddy server to be running in production (or a local Caddy with the
# same config). Tests will fail until prod is switched over.
#
# Usage: ./scripts/test-zecsec-redirects.sh

set -euo pipefail

PASS=0
FAIL=0
ERRORS=""

check_redirect() {
    local url="$1"
    local expected="$2"

    # Follow zero redirects — just grab the Location header from the 301
    location=$(curl -s -o /dev/null -w '%{redirect_url}' --max-redirs 0 "$url" 2>/dev/null || true)

    if [ "$location" = "$expected" ]; then
        printf "  PASS  %s\n" "$url"
        PASS=$((PASS + 1))
    else
        printf "  FAIL  %s\n" "$url"
        printf "        expected: %s\n" "$expected"
        printf "        got:      %s\n" "$location"
        FAIL=$((FAIL + 1))
        ERRORS="${ERRORS}\n  FAIL  ${url} -> got '${location}' expected '${expected}'"
    fi
}

echo "=== Testing zecsec.com redirects ==="
echo ""

# ---------------------------------------------------------------------------
# Posts — bare slug (e.g. /posts/my-first-post)
# ---------------------------------------------------------------------------
echo "--- Posts (bare slug) ---"
check_redirect "https://zecsec.com/posts/my-first-post" \
               "https://defuse.ca/zecsec/my-first-post.htm"
check_redirect "https://zecsec.com/posts/october-update" \
               "https://defuse.ca/zecsec/october-update.htm"
check_redirect "https://zecsec.com/posts/scalable-private-money-needs-scalable-private-messaging" \
               "https://defuse.ca/zecsec/scalable-private-money-needs-scalable-private-messaging.htm"
check_redirect "https://zecsec.com/posts/security-audit-process" \
               "https://defuse.ca/zecsec/security-audit-process.htm"
check_redirect "https://zecsec.com/posts/ywallet-audit-published" \
               "https://defuse.ca/zecsec/ywallet-audit-published.htm"
check_redirect "https://zecsec.com/posts/zecsec-roadmap-for-2023" \
               "https://defuse.ca/zecsec/zecsec-roadmap-for-2023.htm"
check_redirect "https://zecsec.com/posts/2022-q4-transparency-report" \
               "https://defuse.ca/zecsec/2022-q4-transparency-report.htm"
check_redirect "https://zecsec.com/posts/making-zcash-light-wallets-faster-and-more-private" \
               "https://defuse.ca/zecsec/making-zcash-light-wallets-faster-and-more-private.htm"
check_redirect "https://zecsec.com/posts/deep-dive-into-sgx-attacks" \
               "https://defuse.ca/zecsec/deep-dive-into-sgx-attacks.htm"

echo ""

# ---------------------------------------------------------------------------
# Posts — trailing slash (e.g. /posts/my-first-post/)
# ---------------------------------------------------------------------------
echo "--- Posts (trailing slash) ---"
check_redirect "https://zecsec.com/posts/my-first-post/" \
               "https://defuse.ca/zecsec/my-first-post.htm"
check_redirect "https://zecsec.com/posts/october-update/" \
               "https://defuse.ca/zecsec/october-update.htm"
check_redirect "https://zecsec.com/posts/scalable-private-money-needs-scalable-private-messaging/" \
               "https://defuse.ca/zecsec/scalable-private-money-needs-scalable-private-messaging.htm"
check_redirect "https://zecsec.com/posts/security-audit-process/" \
               "https://defuse.ca/zecsec/security-audit-process.htm"
check_redirect "https://zecsec.com/posts/ywallet-audit-published/" \
               "https://defuse.ca/zecsec/ywallet-audit-published.htm"
check_redirect "https://zecsec.com/posts/zecsec-roadmap-for-2023/" \
               "https://defuse.ca/zecsec/zecsec-roadmap-for-2023.htm"
check_redirect "https://zecsec.com/posts/2022-q4-transparency-report/" \
               "https://defuse.ca/zecsec/2022-q4-transparency-report.htm"
check_redirect "https://zecsec.com/posts/making-zcash-light-wallets-faster-and-more-private/" \
               "https://defuse.ca/zecsec/making-zcash-light-wallets-faster-and-more-private.htm"
check_redirect "https://zecsec.com/posts/deep-dive-into-sgx-attacks/" \
               "https://defuse.ca/zecsec/deep-dive-into-sgx-attacks.htm"

echo ""

# ---------------------------------------------------------------------------
# Posts — /index.html suffix (Hugo's canonical format)
# ---------------------------------------------------------------------------
echo "--- Posts (/index.html suffix) ---"
check_redirect "https://zecsec.com/posts/my-first-post/index.html" \
               "https://defuse.ca/zecsec/my-first-post.htm"
check_redirect "https://zecsec.com/posts/october-update/index.html" \
               "https://defuse.ca/zecsec/october-update.htm"
check_redirect "https://zecsec.com/posts/scalable-private-money-needs-scalable-private-messaging/index.html" \
               "https://defuse.ca/zecsec/scalable-private-money-needs-scalable-private-messaging.htm"
check_redirect "https://zecsec.com/posts/security-audit-process/index.html" \
               "https://defuse.ca/zecsec/security-audit-process.htm"
check_redirect "https://zecsec.com/posts/ywallet-audit-published/index.html" \
               "https://defuse.ca/zecsec/ywallet-audit-published.htm"
check_redirect "https://zecsec.com/posts/zecsec-roadmap-for-2023/index.html" \
               "https://defuse.ca/zecsec/zecsec-roadmap-for-2023.htm"
check_redirect "https://zecsec.com/posts/2022-q4-transparency-report/index.html" \
               "https://defuse.ca/zecsec/2022-q4-transparency-report.htm"
check_redirect "https://zecsec.com/posts/making-zcash-light-wallets-faster-and-more-private/index.html" \
               "https://defuse.ca/zecsec/making-zcash-light-wallets-faster-and-more-private.htm"
check_redirect "https://zecsec.com/posts/deep-dive-into-sgx-attacks/index.html" \
               "https://defuse.ca/zecsec/deep-dive-into-sgx-attacks.htm"

echo ""

# ---------------------------------------------------------------------------
# Audit PDFs
# ---------------------------------------------------------------------------
echo "--- Audit PDFs ---"
check_redirect "https://zecsec.com/audits/Free2Z%20Mini%20Audit-Final.pdf" \
               "https://defuse.ca/zecsec/audits/Free2Z%20Mini%20Audit-Final.pdf"
check_redirect "https://zecsec.com/audits/YWalletAuditReport-FINALv3.pdf" \
               "https://defuse.ca/zecsec/audits/YWalletAuditReport-FINALv3.pdf"
check_redirect "https://zecsec.com/audits/zcash-ledger-audit-report-v2.pdf" \
               "https://defuse.ca/zecsec/audits/zcash-ledger-audit-report-v2.pdf"
check_redirect "https://zecsec.com/audits/zecwallet-lite-cli-audit-report-v2.pdf" \
               "https://defuse.ca/zecsec/audits/zecwallet-lite-cli-audit-report-v2.pdf"
check_redirect "https://zecsec.com/audits/ZGo-Security-Audit-v1.1.pdf" \
               "https://defuse.ca/zecsec/audits/ZGo-Security-Audit-v1.1.pdf"

echo ""

# ---------------------------------------------------------------------------
# Images
# ---------------------------------------------------------------------------
echo "--- Images ---"
check_redirect "https://zecsec.com/images/bug-chart.png" \
               "https://defuse.ca/zecsec/images/bug-chart.png"

echo ""

# ---------------------------------------------------------------------------
# Index / listing pages → catch-all to /zecsec.htm
# ---------------------------------------------------------------------------
echo "--- Index / listing pages (catch-all) ---"
check_redirect "https://zecsec.com/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/posts/index.html" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/audits/index.html" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/overview/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/contact/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/categories/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/tags/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/pages/contact/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/pages/contact/index.html" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://zecsec.com/nonexistent-page" \
               "https://defuse.ca/zecsec.htm"

echo ""

# ---------------------------------------------------------------------------
# www subdomain (spot check)
# ---------------------------------------------------------------------------
echo "--- www.zecsec.com (spot check) ---"
check_redirect "https://www.zecsec.com/" \
               "https://defuse.ca/zecsec.htm"
check_redirect "https://www.zecsec.com/posts/my-first-post" \
               "https://defuse.ca/zecsec/my-first-post.htm"
check_redirect "https://www.zecsec.com/audits/ZGo-Security-Audit-v1.1.pdf" \
               "https://defuse.ca/zecsec/audits/ZGo-Security-Audit-v1.1.pdf"

echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo "=== Results ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "Failures:"
    printf "%b\n" "$ERRORS"
    exit 1
fi

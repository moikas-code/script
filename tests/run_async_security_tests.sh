#!/bin/bash
#
# Run all async security tests to validate vulnerability fixes
#

echo "=== Running Async Security Test Suite ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test categories
declare -a test_categories=(
    "async_security_test"
    "async_vulnerability_test"
    "async_transform_security_test"
)

# Run tests by category
total_passed=0
total_failed=0

for category in "${test_categories[@]}"; do
    echo -e "${YELLOW}Running $category tests...${NC}"
    
    if cargo test $category -- --nocapture 2>&1 | tee /tmp/test_output.log; then
        passed=$(grep -c "test result: ok" /tmp/test_output.log || echo 0)
        echo -e "${GREEN}✓ $category: All tests passed${NC}"
        ((total_passed += passed))
    else
        failed=$(grep -c "test result: FAILED" /tmp/test_output.log || echo 1)
        echo -e "${RED}✗ $category: Some tests failed${NC}"
        ((total_failed += failed))
    fi
    echo
done

# Run specific vulnerability tests
echo -e "${YELLOW}Running specific vulnerability tests...${NC}"

declare -a vuln_tests=(
    "test_vuln_01_use_after_free_in_poll_future"
    "test_vuln_02_null_pointer_in_create_future"
    "test_vuln_03_race_condition_in_task_queue"
    "test_vuln_04_memory_exhaustion_attack"
    "test_vuln_05_unbounded_task_spawning"
    "test_vuln_06_recursive_async_exploit"
    "test_vuln_07_shared_state_corruption"
    "test_vuln_08_timeout_bypass_attempt"
    "test_vuln_09_executor_shutdown_race"
    "test_vuln_10_pointer_lifetime_exploit"
)

for test in "${vuln_tests[@]}"; do
    if cargo test $test -- --exact --nocapture &>/dev/null; then
        echo -e "${GREEN}✓ $test${NC}"
        ((total_passed++))
    else
        echo -e "${RED}✗ $test${NC}"
        ((total_failed++))
    fi
done

echo
echo "=== Test Summary ==="
echo -e "Total passed: ${GREEN}$total_passed${NC}"
echo -e "Total failed: ${RED}$total_failed${NC}"

if [ $total_failed -eq 0 ]; then
    echo -e "\n${GREEN}All async security tests passed! ✨${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed. Please check the output above.${NC}"
    exit 1
fi
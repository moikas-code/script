#!/usr/bin/env python3
"""Fix scan_tokens errors by adding .expect() after Lexer::new()"""

import os
import re
import sys

def fix_scan_tokens_in_file(filepath):
    """Fix scan_tokens errors in a single file"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original_content = content
    
    # Pattern to find Lexer::new() calls that are missing .expect()
    # This pattern looks for Lexer::new(something) followed by .scan_tokens()
    pattern = r'let\s+(\w+)\s*=\s*Lexer::new\(([^)]+)\);\s*\n\s*let\s*\([^)]+\)\s*=\s*\1\.scan_tokens\(\)'
    
    # Find all matches
    matches = list(re.finditer(pattern, content, re.MULTILINE))
    
    if not matches:
        return False
    
    # Process matches in reverse order to avoid offset issues
    for match in reversed(matches):
        var_name = match.group(1)
        arg = match.group(2)
        
        # Replace the pattern
        old_text = match.group(0)
        new_text = old_text.replace(
            f"Lexer::new({arg});",
            f'Lexer::new({arg}).expect("Failed to create lexer");'
        )
        
        start = match.start()
        end = match.end()
        content = content[:start] + new_text + content[end:]
    
    # Write back if changes were made
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    
    return False

def main():
    # Find all Rust files in tests directory
    test_files = []
    for root, dirs, files in os.walk('tests'):
        for file in files:
            if file.endswith('.rs'):
                test_files.append(os.path.join(root, file))
    
    # Also check examples
    for root, dirs, files in os.walk('examples'):
        for file in files:
            if file.endswith('.rs'):
                test_files.append(os.path.join(root, file))
    
    fixed_count = 0
    for filepath in test_files:
        if fix_scan_tokens_in_file(filepath):
            print(f"Fixed: {filepath}")
            fixed_count += 1
    
    print(f"\nTotal files fixed: {fixed_count}")

if __name__ == "__main__":
    main()
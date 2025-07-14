#!/usr/bin/env python3
"""
Rust format string fixer utility.

This module provides utilities to fix common Rust format string issues,
especially those that arise when migrating to newer Rust versions.
"""

import re
import os
from pathlib import Path
from typing import List, Tuple, Optional


class RustFormatFixer:
    """Utility class for fixing Rust format string issues."""
    
    def __init__(self):
        self.common_patterns = [
            # Fix inline format arguments (Rust 2021 edition)
            (r'format!\("([^"]*?)\{\}", ([^)]+)\)',
             lambda m: f'format!("{m.group(1)}{{{m.group(2)}}}")'),
            
            # Fix panic! with inline arguments
            (r'panic!\("([^"]*?)\{\}", ([^)]+)\)',
             lambda m: f'panic!("{m.group(1)}{{{m.group(2)}}}")'),
            
            # Fix println! with inline arguments
            (r'println!\("([^"]*?)\{\}", ([^)]+)\)',
             lambda m: f'println!("{m.group(1)}{{{m.group(2)}}}")'),
            
            # Fix eprintln! with inline arguments
            (r'eprintln!\("([^"]*?)\{\}", ([^)]+)\)',
             lambda m: f'eprintln!("{m.group(1)}{{{m.group(2)}}}")'),
        ]
        
        self.error_patterns = [
            # Missing closing parenthesis in Error::key_not_found
            (r'Error::key_not_found\(format!\("([^"]+)", ([^)]+)\)(?!\))',
             r'Error::key_not_found(format!("\1", \2))'),
            
            # Missing closing parenthesis in ok_or_else
            (r'\.ok_or_else\(\|\| Error::key_not_found\(format!\("([^"]+)", ([^)]+)\)(?!\))',
             r'.ok_or_else(|| Error::key_not_found(format!("\1", \2)))'),
            
            # Extra parentheses cleanup
            (r'\)\)\)\)\);', r')));'),
            (r'\)\)\)\);', r'));'),
        ]
    
    def fix_inline_format_args(self, content: str) -> Tuple[str, int]:
        """Fix format strings to use inline format arguments (Rust 2021)."""
        changes = 0
        
        for pattern, replacement in self.common_patterns:
            if callable(replacement):
                new_content = re.sub(pattern, replacement, content)
            else:
                new_content = re.sub(pattern, replacement, content)
            
            if new_content != content:
                changes += len(re.findall(pattern, content))
                content = new_content
        
        return content, changes
    
    def fix_missing_parentheses(self, content: str) -> Tuple[str, int]:
        """Fix missing or extra parentheses in format! calls."""
        changes = 0
        
        for pattern, replacement in self.error_patterns:
            new_content = re.sub(pattern, replacement, content, flags=re.MULTILINE)
            if new_content != content:
                changes += len(re.findall(pattern, content, flags=re.MULTILINE))
                content = new_content
        
        return content, changes
    
    def fix_multiline_format(self, content: str) -> Tuple[str, int]:
        """Fix multiline format! calls with missing parentheses."""
        changes = 0
        lines = content.split('\n')
        
        i = 0
        while i < len(lines):
            line = lines[i]
            
            # Detect start of a multiline format! call
            if 'return Err(Error::' in line and 'format!(' in line:
                # Look for the closing ); in next few lines
                for j in range(i + 1, min(i + 6, len(lines))):
                    if lines[j].strip().endswith(');'):
                        # Check if we need more closing parentheses
                        open_count = 0
                        close_count = 0
                        for k in range(i, j + 1):
                            open_count += lines[k].count('(')
                            close_count += lines[k].count(')')
                        
                        if open_count > close_count:
                            lines[j] = lines[j].replace(');', ')' * (open_count - close_count + 1) + ';')
                            changes += 1
                        break
            
            i += 1
        
        return '\n'.join(lines), changes
    
    def fix_file(self, filepath: str, backup: bool = True) -> bool:
        """Fix format issues in a single Rust file."""
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            
            if backup:
                backup_path = f"{filepath}.backup"
                with open(backup_path, 'w', encoding='utf-8') as f:
                    f.write(content)
            
            # Apply all fixes
            total_changes = 0
            
            content, changes = self.fix_inline_format_args(content)
            total_changes += changes
            
            content, changes = self.fix_missing_parentheses(content)
            total_changes += changes
            
            content, changes = self.fix_multiline_format(content)
            total_changes += changes
            
            if total_changes > 0:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(content)
                print(f"Fixed: {filepath} ({total_changes} changes)")
                return True
            
            return False
            
        except Exception as e:
            print(f"Error processing {filepath}: {e}")
            return False
    
    def fix_directory(self, directory: str = "src", extensions: List[str] = [".rs"]) -> int:
        """Fix format issues in all Rust files in a directory."""
        fixed_files = 0
        
        for root, _, files in os.walk(directory):
            for file in files:
                if any(file.endswith(ext) for ext in extensions):
                    filepath = os.path.join(root, file)
                    if self.fix_file(filepath):
                        fixed_files += 1
        
        return fixed_files
    
    def analyze_format_issues(self, directory: str = "src") -> dict:
        """Analyze format issues without fixing them."""
        issues = {
            'inline_format_args': 0,
            'missing_parentheses': 0,
            'multiline_format': 0,
            'files_with_issues': []
        }
        
        for root, _, files in os.walk(directory):
            for file in files:
                if file.endswith('.rs'):
                    filepath = os.path.join(root, file)
                    try:
                        with open(filepath, 'r', encoding='utf-8') as f:
                            content = f.read()
                        
                        # Check for issues
                        file_has_issues = False
                        
                        for pattern, _ in self.common_patterns:
                            if re.search(pattern, content):
                                issues['inline_format_args'] += 1
                                file_has_issues = True
                        
                        for pattern, _ in self.error_patterns:
                            if re.search(pattern, content):
                                issues['missing_parentheses'] += 1
                                file_has_issues = True
                        
                        if file_has_issues:
                            issues['files_with_issues'].append(filepath)
                            
                    except Exception as e:
                        print(f"Error analyzing {filepath}: {e}")
        
        return issues


def main():
    """Command-line interface for the format fixer."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Fix Rust format string issues")
    parser.add_argument('action', choices=['fix', 'analyze'], 
                        help="Action to perform")
    parser.add_argument('--directory', '-d', default='src',
                        help="Directory to process (default: src)")
    parser.add_argument('--no-backup', action='store_true',
                        help="Don't create backup files")
    parser.add_argument('--file', '-f',
                        help="Fix a specific file instead of directory")
    
    args = parser.parse_args()
    
    fixer = RustFormatFixer()
    
    if args.action == 'analyze':
        issues = fixer.analyze_format_issues(args.directory)
        print("\nFormat Issues Analysis:")
        print(f"  Inline format args needed: {issues['inline_format_args']}")
        print(f"  Missing parentheses: {issues['missing_parentheses']}")
        print(f"  Files with issues: {len(issues['files_with_issues'])}")
        if issues['files_with_issues']:
            print("\nFiles needing fixes:")
            for file in issues['files_with_issues'][:10]:
                print(f"  - {file}")
            if len(issues['files_with_issues']) > 10:
                print(f"  ... and {len(issues['files_with_issues']) - 10} more")
    
    elif args.action == 'fix':
        if args.file:
            success = fixer.fix_file(args.file, backup=not args.no_backup)
            print(f"\nFixed: {success}")
        else:
            fixed = fixer.fix_directory(args.directory)
            print(f"\nFixed {fixed} files")


if __name__ == "__main__":
    main()
#\!/bin/bash

# Script to fix Rust format string syntax errors

# Find all .rs files and fix format strings with old syntax
find src/ -name "*.rs" -type f  < /dev/null |  while read -r file; do
    echo "Processing: $file"
    
    # Use perl for more complex regex replacement
    perl -i.bak -pe 's/format\!\(([^)]*?)\{([^}]*?[a-zA-Z_][a-zA-Z0-9_]*[^}]*?)\}([^)]*?)\)/format\!($1{}$3, $2)/g' "$file"
    
    # Check if changes were made
    if \! cmp -s "$file" "$file.bak"; then
        echo "  Modified: $file"
    else
        echo "  No changes: $file"
        rm "$file.bak"
    fi
done

echo "Format string fix complete"

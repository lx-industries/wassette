#!/usr/bin/env bash
#
# Script to pin GitHub Actions to their commit SHAs in .lock.yml files
# This enhances security by preventing unwanted or malicious updates to third-party actions.
#

set -euo pipefail

echo "Pinning GitHub Actions in .lock.yml files to their commit SHAs..."
echo ""

# Counter for changes
total_replacements=0

# Process each .lock.yml file
for lockfile in .github/workflows/*.lock.yml; do
    if [[ ! -f "$lockfile" ]]; then
        continue
    fi
    
    echo "Processing: $lockfile"
    
    # Use sed to replace all occurrences in one pass
    # actions/checkout@v5 -> v5.0.0
    sed -i 's|uses: actions/checkout@v5|uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0|g' "$lockfile"
    
    # actions/cache@v4 -> v4.2.0
    sed -i 's|uses: actions/cache@v4|uses: actions/cache@87d64cb69bcfab0eca2dc88ebb5b19fdd9e43f58 # v4.2.0|g' "$lockfile"
    
    # actions/setup-node@v4 -> v4.2.0
    sed -i 's|uses: actions/setup-node@v4|uses: actions/setup-node@7fcf203820d60326f0ef36c1e8b2b6c29038dd4b # v4.2.0|g' "$lockfile"
    
    # actions/upload-artifact@v4 -> v4.6.2
    sed -i 's|uses: actions/upload-artifact@v4|uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2|g' "$lockfile"
    
    # actions/download-artifact@v5 -> v5.0.0
    sed -i 's|uses: actions/download-artifact@v5|uses: actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0 # v5.0.0|g' "$lockfile"
    
    # actions/github-script@v8 -> v8.0.0
    sed -i 's|uses: actions/github-script@v8|uses: actions/github-script@ed597411d8f924073f98dfc5c65a23a2325f34cd # v8.0.0|g' "$lockfile"
    
    echo "  ✓ Processed"
done

echo ""
echo "All GitHub Actions have been pinned to their commit SHAs."

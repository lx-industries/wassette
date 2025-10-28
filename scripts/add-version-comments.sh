#!/usr/bin/env bash
#
# Script to add version comments to already-pinned GitHub Actions in .lock.yml files
# This improves readability by indicating which version each SHA corresponds to.
#

set -euo pipefail

echo "Adding version comments to pinned GitHub Actions in .lock.yml files..."
echo ""

# Process each .lock.yml file
for lockfile in .github/workflows/*.lock.yml; do
    if [[ ! -f "$lockfile" ]]; then
        continue
    fi
    
    echo "Processing: $lockfile"
    
    # Add version comments to already-pinned SHAs (if not already present)
    # actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 -> add # v5.0.0
    sed -i 's|uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8$|uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0|g' "$lockfile"
    
    # actions/cache@87d64cb69bcfab0eca2dc88ebb5b19fdd9e43f58 -> add # v4.2.0
    sed -i 's|uses: actions/cache@87d64cb69bcfab0eca2dc88ebb5b19fdd9e43f58$|uses: actions/cache@87d64cb69bcfab0eca2dc88ebb5b19fdd9e43f58 # v4.2.0|g' "$lockfile"
    
    # actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 -> add # v4.3.0
    sed -i 's|uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830$|uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0|g' "$lockfile"
    
    # actions/setup-node@7fcf203820d60326f0ef36c1e8b2b6c29038dd4b -> add # v4.2.0
    sed -i 's|uses: actions/setup-node@7fcf203820d60326f0ef36c1e8b2b6c29038dd4b$|uses: actions/setup-node@7fcf203820d60326f0ef36c1e8b2b6c29038dd4b # v4.2.0|g' "$lockfile"
    
    # actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 -> add # v4.4.0
    sed -i 's|uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020$|uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4.4.0|g' "$lockfile"
    
    # actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 -> add # v4.6.2
    sed -i 's|uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02$|uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2|g' "$lockfile"
    
    # actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0 -> add # v5.0.0
    sed -i 's|uses: actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0$|uses: actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0 # v5.0.0|g' "$lockfile"
    
    # actions/github-script@ed597411d8f924073f98dfc5c65a23a2325f34cd -> add # v8.0.0
    sed -i 's|uses: actions/github-script@ed597411d8f924073f98dfc5c65a23a2325f34cd$|uses: actions/github-script@ed597411d8f924073f98dfc5c65a23a2325f34cd # v8.0.0|g' "$lockfile"
    
    echo "  ✓ Processed"
done

echo ""
echo "All pinned GitHub Actions now have version comments."

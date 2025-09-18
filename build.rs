// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

use std::fs;

fn main() {
    // Ensure the build script reruns when the Git HEAD moves so we embed fresh metadata.
    println!("cargo:rerun-if-changed=.git/HEAD");

    if let Ok(head_ref) = fs::read_to_string(".git/HEAD") {
        if let Some(reference) = head_ref.strip_prefix("ref: ").map(str::trim) {
            println!("cargo:rerun-if-changed=.git/{reference}");
        }
    }

    built::write_built_file().expect("Failed to acquire build-time information");
}

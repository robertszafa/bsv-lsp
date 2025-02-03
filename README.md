This is an early attempt at a language server (LS) for Bluespec System Verilog. It's very much bare bones, with line diagnostics being the only feature implemented at the moment. In fact, all this code is doing at the moment is invoking the `bsc` compiler, parsing its output, and sending diagnostic to the LSP client to display errors/warnings in the editor on file save.

This LS is using the LSP implementation from the tower-lsp Rust crate.

Easy things to do next:
- Workspace/project support. Currently, the LS works on single files. Bluespec does not have a standard packaging system, like a Cargo.toml file in Rust, so LS support for workspaces would involve settling on a similar format (e.g., using an `LSP` make rule from some root Makefile that specifies to the LS what compile commands should be ran, including any dependency files).
- Support for LSP didChange events, instead of updating only on didSave.
- goto definition support. This might be implemented using `bluetcl` hacks, or I might put in the work to create a tree-sitter-bsv grammar. 

tree-sitter-bsv grammar: 
- This would make a lot of things easier, e.g., goto definitions, context-sensitive completions, etc. This is a bigger project than this LS, and would be beneficial in other places, e.g., automatic indentation and syntax highlighting in most editors.




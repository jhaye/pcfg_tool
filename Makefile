pcfg_tool:
	cargo build --release
	cp target/release/pcfg_tool pcfg_tool
	chmod +x pcfg_tool

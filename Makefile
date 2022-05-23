pcfg_tool:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	cp target/release/pcfg_tool pcfg_tool
	chmod +x pcfg_tool

clean:
	rm pcfg_tool

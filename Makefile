target := x86_64-unknown-none
profile := debug
outdir := target/$(target)/$(profile)/

$(outdir)myros.iso: grub.cfg $(outdir)myros cargo-$(profile)
	mkdir -p $(outdir)iso/boot/grub/
	cp $(outdir)myros $(outdir)iso/boot/
	cp grub.cfg $(outdir)iso/boot/grub/
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(outdir)myros.iso $(outdir)iso/

$(outdir)myros: cargo-$(profile)

cargo-debug:
	cargo xbuild --target $(target).json
cargo-release:
	cargo xbuild --release --target $(target).json

clean:
	cargo clean --target-dir target/$(target)
all-clean:
	cargo clean

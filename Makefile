#   Copyright 2020 Michael Leany
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#       http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.
###################################################################################################

target := x86_64-unknown-none
profile := debug
outdir := target/$(target)/$(profile)/
release-flag := --release
flags := $($(profile)-flag) --target $(target).json -Z build-std=core,alloc,compiler_builtins

$(outdir)myros.iso: grub.cfg $(outdir)myros
	mkdir -p $(outdir)iso/boot/grub/
	cp $(outdir)myros $(outdir)iso/boot/
	cp grub.cfg $(outdir)iso/boot/grub/
	grub-mkrescue -d /usr/lib/grub/i386-pc/ -o $(outdir)myros.iso $(outdir)iso/

$(outdir)myros:
	cargo build $(flags)
	cargo clippy $(flags)
	cargo doc $(flags)

clean:
	cargo clean

memory := 1024
run: $(outdir)myros.iso
	qemu-system-x86_64 -boot d -cdrom $(outdir)myros.iso -m $(memory)

# include dependency files created by cargo
-include $(outdir)*.d

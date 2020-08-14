---
title: "1. \"Hello, world!\""
layout: page
nav_order: 1
last_modified_date: 2020-08-11
permalink: /posts/hello-world/
---

# "Hello, world!"

---

The goal of this post, is to develop and boot to a stand-alone executable which will display the message "Hello, world!" on the screen. The key points that you should learn from this post are the following:

- How to cross compile your kernel or other program as a stand-alone executable
- The booting process
- How to link with external code
- The minimum setup to create a Multiboot2-compliant kernel
- How to test your operating system

We'll start off by executing `git new myros`, which will create **Cargo.toml** and **src/main.rs**

## Cross Compiling

Typically when we run cargo, it will compile our Rust code for whichever host operating system we're using at the time. For most people that will be Windows, Linux, or Mac OS. However, this executable needs to run independently of any existing operating system. For that reason, it needs to be cross compiled for a custom target: x86_64-unknown-none. That can be done with the following steps.

### Using build-std

First, since this is a custom target, we need to compile some built-in crates (such as `core`) for this target. To do that, we can use a new, but unstable cargo feature called [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std). Previously this project used an external tool called [cargo-xbuild](https://docs.rs/crate/cargo-xbuild/), but with build-std being added to cargo itself, it has become the recommended solution.

To use build-std, we need to install rust-src with the following command:
```
rustup component add rust-src
```

With rust-src installed, we can build this project with the following command, where x86_64-unknown-none.json defines the target system:

```
cargo build -Z build-std=core,compiler_builtins,alloc --target=x86_64-unknown-none.json
```

To avoid typing `-Z build-std=core,compiler_builtins,alloc` every time we build, we can create a file called **.cargo/config.toml** with the following contents:

```toml
[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
```

We would still need to specify the target, so the resulting command would be:

```
cargo build --target=x86_64-unknown-none.json
```

### Defining the Target
Of course, to use the command given above we first have to define the target in **x86_64-unknown-none.json**. Below are its contents:

```json
{
  "llvm-target": "x86_64-unknown-none",
  "data-layout": "e-m:e-i64:64-f80:128-n8:16:32:64-S128",
  "arch": "x86_64",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "os": "none",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "pre-link-args": {
    "ld.lld": [
      "--image-base=0x100000"
    ]
  },
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float"
}
```

Here I will only briefly describe a few of these options. For more information see the "[Target Specification](https://os.phil-opp.com/minimal-rust-kernel/#target-specification)" section in Phil Oppermann's blog post "[A Minimal Rust Kernel](https://os.phil-opp.com/minimal-rust-kernel/)".

With the `linker-flavor` and `linker` options we indicate that we're using the cross-platform linker LLD. We use the `panic-strategy` option to tell the linker to abort on panic as we don't have support for stack unwinding. We set `disable-redzone`, because otherwise the stack will be corrupted when we handle interrupts. Finally, we pass the argument `--image-base=0x100000` to the linker to set the executable to a fixed base address at 1 MiB.

### Using a Nightly Release of Rust

Finally, since build-std is an unstable feature, we also need to use a nightly release of Rust. Specifically we need Rust nightly 2020–07–15 or later, so make sure you run `rustup update` if you have an older version. One way to ensure the nightly release is used is with a file called **rust-toolchain** which simply contains the following line:

```
nightly
```

## A Dependency-Free Binary

### Removing External Dependencies

To write a stand-alone executable, one that can run without a host operating system, we need to avoid using the `std` crate as well as the Rust startup code, both of which depend on operating system services. To disable the `std` crate, we use the `#![no_std]` attribute. To disable the startup code we use `#![no_main]`. So for now, **src/main.rs** looks like this:

```rust
#![no_std]
#![no_main]
```

Note the absence of a main function. This is, of course, because we used `#![no_main]`, but that leaves our program without an entry point. We'll get back to that later.

### The Panic Handler

If we try to build the code above, we will get the following error:

```
error: `#[panic_handler]` function required, but not found
```

This error is because the panic handler is part of the `std` crate. To resolve this error, we need to implement our own panic handler. I've done that by adding the following code to **src/main.rs**.

```rust
use core::panic::PanicInfo;

/// Replaces the panic handler from the standard library which is not available
/// when using `#![no_std]` in a binary.
///
/// Does not return.
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    halt();
}

/// Halt execution. If halting due to a failure, use `panic` instead.
///
/// Does not return.
pub fn halt() -> ! {
    loop {}
}
```

For now, the panic handler doesn't do anything interesting. Eventually we'll want it to display a panic message on the screen, but this works for now.

### Building the Binary

This will now build without giving any errors However, despite the lack of error messages, we still have a problem. See the below output from objdump:

```
$ objdump -fh target/x86_64-unknown-none/debug/myros

target/x86_64-unknown-none/debug/myros:     file format elf64-x86-64
architecture: i386:x86-64, flags 0x00000112:
EXEC_P, HAS_SYMS, D_PAGED
start address 0x0000000000000000

Sections:
Idx Name          Size      VMA               LMA               File off  Algn
  0 .comment      00000012  0000000000000000  0000000000000000  000000e8  2**0
```

Note that the only section is `.comment`. Code generally appears in a `.text` section, but that isn't there, and the start address is 0x0000000000000000 (null). This binary isn't really executable as there is no executable code and, as we pointed out earlier, no entry point. The linker is expecting a function called `_start` (with no name mangling) to be our entry point. We will need to create that function, but first let's look into what it will take to get our executable loaded and running without the help of an underlying operating system.

## Booting

Writing a complete bootloader for an x86-64 CPU is a pretty tedious task due to the legacy modes that exist for backward compatibility. To load a 64-bit executable, such as a kernel, we would need to write a 16-bit first-stage loader which must be contained entirely within the first sector (512 bytes) of a disk or partition along with data about the disk or partition, such as a master boot record or file system information. This first stage must be able to locate and load a second stage from disk since it may be impossible to meet all the other system intializaton requirements in less than 512 bytes of binary code.

The computer starts in Real Mode (the legacy 16-bit mode). From there we would need to do a bit of set up and enter Protected Mode (the legacy 32-bit mode). Once in Protected Mode, we would need to do some additional set up to finally enter Long Mode (64-bit mode). We could then locate and load our 64-bit executable from disk.

The entire first stage and much of the second stage would have to be written in assembly, including 16-bit, 32-bit and 64-bit varieties. Depending on how the second stage and our final executable are stored on disk we may need two file system drivers, one of which must be written entirely in assembly and fit within the 512-byte first stage.

While writing a bootloader is a valuable learning exercise, I think it's more valuable to dive into the guts of the operating system intially and worry about things like writing bootloaders when you have more experience. Fortunately most of the work of writing a bootloader has already been done for us.

We will be using a bootloader called GRUB. To take advantage of it, we need to comply with the Multiboot specification, specifically [Multiboot2](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html).

### External Assembly Code

GRUB will do the vast majority of the work needed to initialize the system and load our executable. However, there is one thing that it won't do: set up Long Mode (unless we use EFI, which we won't). It will take us as far as Protected Mode and load our executable, but we need to set up Long Mode ourselves. That means that while we will have a 64-bit executable, we need to have a 32-bit entry point (the `_start` function).

I have chosen to solve this problem by writing the `_start` function in assembly. It is in the file **src/arch_x86_64/boot.asm**, which uses [NASM](https://www.nasm.us) syntax. I will explain how to get to Long Mode in a later post. For now, let's just print "Hello, world!" to the screen, like so:

```nasm
section .rodata align=16
MSG:
    db  "Hello, world!", 0

[BITS 32]
section .text
[global _start]
_start:
    lea edx, [0xb8000]      ; video memory base address
    lea ebx, [MSG]          ; our message
    mov ecx, 0              ; byte offset
    mov ah, 0x0f            ; color - white on black
.loop:
    mov al, [ebx+ecx]       ; read the next character
    cmp al, 0               ; if it's null
    jz .loop_end            ; break
    mov [edx+ecx*2], ax     ; write the character (with the color) to the screen
    inc ecx                 ; increment the byte offset
    jmp .loop
.loop_end:

.halt:
    hlt
    jmp .halt
.end:
```

We now need to tell cargo to assemble and inlcude **boot.asm**. To assemble it, we use a [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html). The build script, by default is named **build.rs** and is located in the project's root directory, not in the **src/** directory. It is Rust code that is built and run before building the project. It can be used to prepare external dependencies like our assembly code.

In our build script we will use the crate [`nasm-rs`](https://crates.io/crates/nasm-rs) to assemble **boot.asm** with NASM. To include `nasm-rs` for use in our build script we need to add the following lines to **Cargo.toml**:

```toml
[build-dependencies]
nasm-rs = "0.1.7"
```

Here is **build.rs**, which assembles and links **src/arch_x86_64/boot.asm** into the library **libboot.a** using the ELF64 object file type.

```rust
use nasm_rs;

fn main() {
    let src = "src/arch_x86_64/boot.asm";
    println!("cargo:rerun-if-changed={}", src);
    nasm_rs::compile_library_args("libboot.a", &[src], &["-felf64"]);
}
```

We now need to tell Rust to include the library **libboot.a** and the `_start` function. We do that by adding the following code to **src/main.rs**.

```rust
#[link(name = "boot", kind = "static")]
extern "C" {
    /// kernel's entry point, implemented in src/arch_x86_64/boot.asm
    fn _start() -> !;
}
```

The `link` attribute tells Rust to staticly link with the `boot` library (which is located in **libboot.a**) and that the symbols in the following `extern` block are located in that library. `extern "C"` means that these symbols use the C ABI.

### The Multiboot Header

With the additions in the previous section, we can now build a real executable, but GRUB doesn't know how to load it. The Multiboot specification requires a [header](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Header-layout) as follows. 

```
Offset  Type    Field Name      Note
0       u32     magic           required
4       u32     architecture    required
8       u32     header_length   required
12      u32     checksum        required
16-XX           tags            required
```

We will use the [flags tag](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Console-header-tags) and the required [end tag](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Header-tags). These have the following layouts.

```
Flags tag
        +-------------------+
u16     | type = 4          |
u16     | flags             |
u32     | size = 12         |
u32     | console_flags     |
        +-------------------+

End tag
        +-------------------+
u16     | type = 0          |
u16     | flags             |
u32     | size = 8          |
        +-------------------+
```

To comply with these specificatons, we add the following code to the top of **boot.asm**:

```nasm
MBH_MAGIC       equ  0xE85250D6
MBH_LENGTH      equ  (_MB_HEADER.end - _MB_HEADER)
MBH_CHECKSUM    equ  -(MBH_MAGIC + MBH_LENGTH) & 0xFFFFFFFF

section .rodata align=16
_MB_HEADER:
    dd   MBH_MAGIC          ; magic
    dd   0                  ; architechture (i386 - protected mode)
    dd   MBH_LENGTH         ; header length
    dd   MBH_CHECKSUM       ; checksum

align  8
.flags_tag:
    dw   4                  ; type = 4 (flags tag)
    dw   0                  ; flags
    dd   12                 ; size
    dd   3                  ; console flags (EGA console)

align  8
.end_tag:
    dw   0                  ; type = 0 (end tag)
    dw   0                  ; flags
    dd   8                  ; size
.end:
```

If we were linking to a non-ELF object file format, we would also need to include tags for the entry point and the size of the `.bss` section, etc. However, since we're using ELF64, GRUB can determine all that automatically.

## Finishing Touches

Now we have everything we need to build our completed Multiboot2-compliant "Hello, world!" executable. Now we just need a way to test it. We first need to create a bootable ISO, then we can run it with Qemu, VirtualBox or some other virtual machine. Or, if you really want to, you can test it on real hardware. But be careful; the wrong kind of mistake could do physical damage.

### Creating a Bootable ISO (on Linux)

I apologize to those readers who are using a system other than Linux (and possibly even those with non-Debian-based distributions of Linux). You may need to do some research on your own to complete this task.

To create a bootable ISO with GRUB as the bootloader, we need a GRUB configuration file. Mine looks like this:

```
menuentry Myros {
    multiboot2 /boot/myros
}
```

It adds an entry in the GRUB  menu called "Myros", and specifies that when that option is selected, it should load **/boot/myros** as a Multiboot2 executable.

Now we run the following commands in the terminal:

```
mkdir -p target/x86_64-unknown-none/debug/iso/boot/grub/
cp target/x86_64-unknown-none/debug/myros target/x86_64-unknown-none/debug/iso/boot/
cp grub.cfg target/x86_64-unknown-none/debug/iso/boot/grub/
grub-mkrescue -d /usr/lib/grub/i386-pc -o target/x86_64-unknown-none/debug/myros.iso target/x86_64-unknown-none/debug/iso/
```

This creates the directory **target/x86_64-unknown-none/debug/iso/boot/grub/**, along with all parent directories that don't yet exist. It then copies our executable (**target/x86_64-unknown-none/debug/myros**) and **grub.cfg** into this directory structure.

Finally, the `grub-mkrescue` command creates a new ISO with the directory structure under **target/x86_64-unknown-none/debug/iso/** and with GRUB installed.

If this command fails, here are a couple of things to check:
- Is `xorriso` installed on your system? If not, and assuming you have root/sudo access, then you need to install it: `sudo apt install xorriso` on Debian-based distributions.
- Does **/usr/lib/grub/i386-pc** exist? You can install it with `sudo apt install grub-pc-bin` on Debian-based distributions.

To make executing these steps a bit easier, and to allow for easy cleanup, I have created the following **Makefile**:

```make
target := x86_64-unknown-none
profile := debug
outdir := target/$(target)/$(profile)/
release-flag := --release
flags := $($(profile)-flag) --target $(target).json

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

# include dependency files created by cargo
-include $(outdir)*.d
```

To build the executable and create the ISO we would execute one of the following commands

```bash
make # defaults to debug profile
make profile=debug
make profile=release
```

### Testing with Qemu

[Qemu](https://www.qemu.org/) is an emulator which is commonly used for testing operating system code. On Debian-based Linux distributions, it can be installed with the following command:

```
sudo apt install qemu
```

It can be run with the following command (for debug). The `-m` option allows you to specify how much memory your virtual machine should have, in megabytes. The default is 128. You can adjust this number to fit your needs.

```
qemu-system-x86_64 -boot d -cdrom target/x86_64-unknown-none/debug/myros.iso -m 1024
```

We can add this to our **Makefile** like so:

```make
memory := 1024
run: $(outdir)myros.iso
	qemu-system-x86_64 -boot d -cdrom $(outdir)myros.iso -m $(memory)
```

Here are some examples of how we can use `make` to run the job:

```bash
make run # runs the debug target with 1024 MiB of RAM
make run profile=debug # same as the above
make run profile=release # runs the release target instead
make run memory=4096 # runs with 4096 MiB of RAM
make run profile=release memory=256 # runs the release target with 256 MiB of RAM
```

If your test succeeded and you see "Hello, world!" printed on the screen, congratulations! You've just completed what may be the most complicated "Hello, world!" program you will ever write.

---

Next: [2. Long Mode](/posts/long-mode) &raquo;&raquo;&raquo;

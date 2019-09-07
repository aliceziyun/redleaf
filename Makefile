
arch ?= x86_64
kernel := build/kernel.bin
iso := build/redleaf.iso

linker_script := linker.ld
grub_cfg := boot/grub.cfg
#assembly_source_files := $(wildcard src/*.asm)
#assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

target ?= $(arch)-redleaf
rust_os := target/$(target)/debug/libredleaf.a

.PHONY: all clean run iso kernel doc disk

all: $(kernel)

release: $(releaseKernel)

clean:
	rm -r build
	cargo clean

# To trace interrupts add: -d int,cpu_reset

run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log -no-reboot -no-shutdown -d int,cpu_reset -smp 2

run-nox: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log -no-reboot -nographic -d int,cpu_reset -smp 2

iso: $(iso)
	@echo "Done"

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) bootblock entryother $(linker_script) 
	ld -n --gc-sections -T $(linker_script) -o $(kernel) build/boot.o build/multiboot_header.o $(rust_os) -b binary build/entryother.bin

kernel:
	@RUST_TARGET_PATH=$(32shell pwd) cargo xbuild --target x86_64-redleaf.json

# compile assembly files
bootblock: src/boot.asm src/multiboot_header.asm
	@mkdir -p build
	nasm -felf64 src/boot.asm -o build/boot.o
	nasm -felf64 src/multiboot_header.asm -o build/multiboot_header.o

# compile assembly files
entryother: src/entryother.asm
	@mkdir -p build
	nasm -felf64 src/entryother.asm -o build/entryother.o
	ld -N -e start_others16 -Ttext 0x7000 -o build/entryother.out build/entryother.o
	objcopy -S -O binary -j .text build/entryother.out build/entryother.bin


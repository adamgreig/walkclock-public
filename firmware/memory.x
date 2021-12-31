MEMORY {
  FLASH   : ORIGIN = 0x08000000, LENGTH = 1M
  RAM     : ORIGIN = 0x20000000, LENGTH = 128K
  AXISRAM : ORIGIN = 0x24000000, LENGTH = 512K
  SRAM1   : ORIGIN = 0x30000000, LENGTH = 128K
  SRAM2   : ORIGIN = 0x30020000, LENGTH = 128K
  SRAM3   : ORIGIN = 0x30040000, LENGTH = 32K
  SRAM4   : ORIGIN = 0x38000000, LENGTH = 64K
  BSRAM   : ORIGIN = 0x38800000, LENGTH = 4K
  ITCM    : ORIGIN = 0x00000000, LENGTH = 64K
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS {
  .axisram (NOLOAD) : ALIGN(8) {
    *(.axisram .axisram.*);
    . = ALIGN(8);
  } > AXISRAM
  .sram1 (NOLOAD) : ALIGN(4) {
    *(.sram1 .sram1.*);
    . = ALIGN(4);
  } > SRAM1
  .sram2 (NOLOAD) : ALIGN(4) {
    *(.sram2 .sram2.*);
    . = ALIGN(4);
  } > SRAM2
  .sram3 (NOLOAD) : ALIGN(4) {
    *(.sram3 .sram3.*);
    . = ALIGN(4);
  } > SRAM3
  .sram4 (NOLOAD) : ALIGN(4) {
    *(.sram4 .sram4.*);
    . = ALIGN(4);
  } > SRAM4
  .bsram (NOLOAD) : ALIGN(4) {
    *(.bsram .bsram.*);
    . = ALIGN(4);
  } > BSRAM
};

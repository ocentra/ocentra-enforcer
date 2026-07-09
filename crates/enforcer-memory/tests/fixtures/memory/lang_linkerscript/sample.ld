ENTRY(_start)

SECTIONS
{
  . = ASSERT(DEFINED(ORIGIN), "origin must be defined");
  .text : { *(.text) }
}

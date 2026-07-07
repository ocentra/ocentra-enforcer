%include "common.inc"

%define BUFSIZE 128

section .text
global _start

_start:
    mov eax, 1
    call print_msg
    cmp eax, 0
    je exit

print_msg:
    mov edx, 5
    ret

exit:
    mov eax, 60
    syscall

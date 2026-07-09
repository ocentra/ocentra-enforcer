section .text
global main

main:
    mov eax, 1
    call foo
    ret

foo:
    ret

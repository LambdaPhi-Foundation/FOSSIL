declare i32 @puts(i8*)
declare i32 @putchar(i32)

define i32 @main() {
entry:
    call void asm sideeffect "nop", ""()
    ret i32 0
}


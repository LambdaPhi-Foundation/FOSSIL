declare i32 @puts(i8*)
declare i32 @putchar(i32)

define i32 @main() {
entry:
    %0 = alloca i32
    store i32 10, ptr %0
    %1 = alloca i32
    store i32 20, ptr %1
    %2 = alloca i32
    store i32 0, ptr %2
    %3 = load i32, ptr %0
    %4 = load i32, ptr %1
    %5 = load i32, ptr %2
    call void asm sideeffect "mov rax, $0\\0Aadd rax, $1\\0Amov $2, rax", "r,r,r,~{dirflag},~{fpsr},~{flags}#intel"(i32 %3, i32 %4, i32 %5)
    %6 = load i32, ptr %2
    %7 = icmp eq i32 %6, 30
    br i1 %7, label %then.0, label %endif.1
then.0:
    ret i32 0
endif.1:
    ret i32 1
}


define i32 @main() {
entry:
  %1 = call i32 @foo(i32 42)
  br label %exit
exit:
  ret i32 %1
}

declare i32 @foo(i32)

@global_var = global i32 0

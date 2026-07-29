.class public LFoo;
.super Ljava/lang/Object;

.method public add(II)I
    .locals 1
    add-int v0, p1, p2
    return v0
.end method

.method public static main([Ljava/lang/String;)V
    .locals 0
    invoke-static {}, LFoo;->add(II)I
    return-void
.end method

namespace Widgets

open System.Text

type Animal(name: string) =
    member this.Name = name

type Widget(name: string) =
    inherit Animal(name)

    member this.Draw() =
        if name = "" then
            printfn "unnamed"
        helper name

let helper label =
    printfn "helper"

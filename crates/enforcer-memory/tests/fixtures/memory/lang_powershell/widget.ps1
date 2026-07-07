using namespace System.Collections.Generic

class Animal {
    [string]$Name

    [void] Speak() {
        Write-Host "..."
    }
}

class Dog : Animal {
    [void] Speak() {
        Write-Host "Woof"
        Helper
    }
}

function Helper {
    Add-Numbers 1 2
}

function Add-Numbers {
    param([int]$a, [int]$b)
    if ($a -gt 0) {
        return $a + $b
    } else {
        throw "bad"
    }
}

$d = New-Object Dog
$d.Speak()
Helper

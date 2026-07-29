include "llvm/Target/Target.td"

class Instruction {
  string Name;
}

def MyInstr : Instruction {
  let Name = "myinstr";
}

multiclass Foo<string name> {
  def _rr : Instruction;
}

defm Bar : Foo<"bar">;

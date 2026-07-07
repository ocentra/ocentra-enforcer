unit Widget;

interface

uses SysUtils, Classes;

type
  TAnimal = class
  end;

  TDog = class(TAnimal, IFoo)
  private
    FName: string;
  public
    property Name: string read FName write FName;
    procedure Bark;
  end;

procedure DoWork;

implementation

procedure DoWork;
begin
  Helper(1, 2);
end;

procedure TDog.Bark;
begin
  Helper(1);
  Obj.Draw;
end;

end.

with Ada.Text_IO; use Ada.Text_IO;

package body Widget is

   type Animal_Type is record
      Name : String (1 .. 10);
   end record;

   type Dog_Type is new Animal_Type;

   function Helper (Label : String) return String is
   begin
      if Label'Length = 0 then
         return "unnamed";
      end if;
      return Label;
   end Helper;

   procedure Draw is
   begin
      Put_Line ("drawing");
      Helper ("x");
   end Draw;

end Widget;

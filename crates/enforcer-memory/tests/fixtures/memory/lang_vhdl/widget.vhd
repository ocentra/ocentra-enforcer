library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity widget is
end entity widget;

architecture rtl of widget is
  function helper(x : integer) return integer is
  begin
    return x + 1;
  end function helper;
begin
  inst1: component_name port map (a => b);
  process
  begin
    if helper(1) > 0 then
      report "ok";
    end if;
  end process;
end architecture rtl;

import mypkg::*;

module widget;
  function int helper(int x);
    return x + 1;
  endfunction

  class Area extends Base;
    function int compute(int shape);
      return helper(shape);
    endfunction
  endclass

  initial begin
    helper(1);
    $display("ok");
  end
endmodule

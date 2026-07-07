module widget;
  function integer helper;
    input integer x;
    begin
      helper = x + 1;
    end
  endfunction

  initial begin
    if (helper(1))
      $display("ok");
  end
endmodule

module src (output wire [31:0] out);
  assign out = 32'hFFFFFFFF;
endmodule
module top;
  wire [31:0] arr [0:7];
  src u7(.out(arr[7]));
  initial begin
    #1;
    $vogls_assert_eq(arr[7], 32'hFFFFFFFF);
  end
endmodule

// vogls: verify-vcd
module \fodiator_test::add_one (input_i, output__);
  input [3:0] input_i;
  wire [3:0] input_i;
  output [4:0] output__;
  wire [4:0] output__;
  wire [4:0] _0_;
  wire [4:0] _e_1730;
  wire [3:0] \input ;
  assign _0_ = \input  + 4'h1;
  assign \input  = input_i;
  assign _e_1730 = _0_;
  assign output__ = _e_1730;
endmodule

module tb();
    reg [3:0] i;
    wire [4:0] o;
    \fodiator_test::add_one x(i, o);

    initial begin
        #1
        i = 2;
        #1
        $vogls_assert_eq(o, 5'h03);
        i = 3;
        #1
        $vogls_assert_eq(o, 5'h04);
    end
endmodule

module dut (output wire [7:0] w0, output wire [8:0] w1);

   localparam ZERO = 0;
   localparam ONE  = 1;

   assign w0 = {{ZERO{1'b0}}, 8'hA5};
   assign w1 = {{ONE{1'b0}}, 8'hA5};

endmodule

module tb;
   wire [7:0] w0;
   wire [8:0] w1;

   dut u_dut (.w0(w0), .w1(w1));

   initial begin
      #1;
      $vogls_assert_eq(w0, 'hA5);
      $vogls_assert_eq(w1, 'hA5);
   end
endmodule

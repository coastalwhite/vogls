module y
#( parameter integer N = 0 )
(output [3:0] out);
    assign out = N;
endmodule

module z();
    wire [3:0] out;

    y #(.N(7)) y7  (out );

    initial begin
        #1 $vogls_assert_eq(out, 4'b0111);
    end
endmodule

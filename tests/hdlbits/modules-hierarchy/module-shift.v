module my_dff ( input clk, input d, output reg q );
    always @(posedge clk) begin
        q <= d;
    end
endmodule

module top_module ( input clk, input d, output q );
    wire q1, q2;

    my_dff i1 ( clk, d,  q1 );
    my_dff i2 ( clk, q1, q2 );
    my_dff i3 ( clk, q2, q  );
endmodule

module tb();
    reg clk, d;
    wire q;

    top_module m(clk, d, q);

    always #1 clk = ~clk;
    initial begin
        #0 clk = 0; d = 0; #6
        #2 d = 1; $vogls_assert_eq(q, 0);
        #2 d = 0; $vogls_assert_eq(q, 0);
        #2 d = 1; $vogls_assert_eq(q, 0);
        #2 $vogls_assert_eq(q, 1);
        #2 $vogls_assert_eq(q, 0);
        #2 $vogls_assert_eq(q, 1);
        #2 $vogls_assert_eq(q, 1);

        $finish();
    end
endmodule

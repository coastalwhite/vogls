module my_dff8 ( input clk, input [7:0] d, output reg [7:0] q );
    always @(posedge clk) begin
        q <= d;
    end
endmodule

module top_module ( input clk, input [7:0] d, input [1:0] sel, output [7:0] q );
    wire [7:0] q1, q2, q3;

    my_dff8 i1 ( clk, d,  q1 );
    my_dff8 i2 ( clk, q1, q2 );
    my_dff8 i3 ( clk, q2, q3 );

    always @(*) begin
        case (sel)
            2'b00: q = d;
            2'b01: q = q1;
            2'b10: q = q2;
            2'b11: q = q3;
        endcase
    end
endmodule

module tb();
    reg clk;
    reg [7:0] d;
    reg [1:0] sel;
    wire [7:0] q;

    top_module m(clk, d, sel, q);

    always #1 clk = ~clk;
    initial begin
        #0 clk = 0; sel = 0; d = 8'h00; #6
        #2 d = 8'hAB;
        begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'hAB);
            sel = 2'b01; #0 $vogls_assert_eq(q, 0);
            sel = 2'b10; #0 $vogls_assert_eq(q, 0);
            sel = 2'b11; #0 $vogls_assert_eq(q, 0);
        end
        #2 d = 8'h12; $vogls_assert_eq(q, 0);
        begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h12);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'hAB);
            sel = 2'b10; #0 $vogls_assert_eq(q, 0);
            sel = 2'b11; #0 $vogls_assert_eq(q, 0);
        end
        #2 d = 8'h23; $vogls_assert_eq(q, 0);
        begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'h12);
            sel = 2'b10; #0 $vogls_assert_eq(q, 8'hAB);
            sel = 2'b11; #0 $vogls_assert_eq(q, 0);
        end
        #2 begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b10; #0 $vogls_assert_eq(q, 8'h12);
            sel = 2'b11; #0 $vogls_assert_eq(q, 8'hAB);
        end
        #2 begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b10; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b11; #0 $vogls_assert_eq(q, 8'h12);
        end
        #2 begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b10; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b11; #0 $vogls_assert_eq(q, 8'h23);
        end
        #2 begin
            sel = 2'b00; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b01; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b10; #0 $vogls_assert_eq(q, 8'h23);
            sel = 2'b11; #0 $vogls_assert_eq(q, 8'h23);
        end

        $finish();
    end
endmodule

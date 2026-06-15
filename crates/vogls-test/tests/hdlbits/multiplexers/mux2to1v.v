module top_module( 
    input [15:0] a, b, c, d, e, f, g, h, i,
    input [3:0] sel,
    output [15:0] out );
    
    always @(*) begin
        case (sel)
            4'b0000: out = a;
            4'b0001: out = b;
            4'b0010: out = c;
            4'b0011: out = d;
            4'b0100: out = e;
            4'b0101: out = f;
            4'b0110: out = g;
            4'b0111: out = h;
            4'b1000: out = i;
            default: out = 16'hFFFF;
        endcase
    end
endmodule

module tb();
    reg [15:0] a, b, c, d, e, f, g, h, i;
    reg [3:0] sel;
    wire [15:0] out;

    top_module m(a, b, c, d, e, f, g, h, i, sel, out);

    initial begin
        #0
        a=1;b=42;c=13;d=37;e=78;f=21;g=81;h=93;i=32;
        
        sel = 1; #1 $vogls_assert_eq(out, 42);
        sel = 7; #1 $vogls_assert_eq(out, 93);
        sel = 5; #1 $vogls_assert_eq(out, 21);
    end
endmodule

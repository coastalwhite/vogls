module top_module ( 
    input [15:0] scancode,
    output reg left,
    output reg down,
    output reg right,
    output reg up
);
    always @(*) begin
        left = 0; down = 0; right = 0; up = 0;
        case (scancode)
            16'he06b: left  = 1;
            16'he072: down  = 1;
            16'he074: right = 1;
            16'he075: up    = 1;
        endcase
    end

endmodule

module tb();
    reg [16:0] scancode;
    wire left, down, right, up;

    top_module m(scancode, left, down, right, up);

    initial begin
        scancode = 16'h0;
        // #1 $vogls_assert_eq(|{left, down, right, up}, 0); scancode = 16'h1; 
        // #1 $vogls_assert_eq(|{left, down, right, up}, 0); scancode = 16'he06b; 
        // #1 $vogls_assert_eq(left, 1); $vogls_assert_eq(|{down, right, up}, 0); scancode = 16'he072; 
        // #1 $vogls_assert_eq(down, 1); $vogls_assert_eq(|{left, right, up}, 0); scancode = 16'he074; 
        // #1 $vogls_assert_eq(right, 1); $vogls_assert_eq(|{left, down, up}, 0); scancode = 16'he075; 
        // #1 $vogls_assert_eq(up, 1); $vogls_assert_eq(|{left, down, right}, 0); scancode = 16'h0; 
        // #1 $vogls_assert_eq(|{left, down, right, up}, 0); 
    end
endmodule

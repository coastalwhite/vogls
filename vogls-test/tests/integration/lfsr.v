// vogls: timeout=100
module lfsr #(
    parameter WIDTH = 8
) (
    input wire clk,
    input wire reset,

    output wire [WIDTH-1:0] bits
);
    reg [16 + WIDTH - 1:0] shift_reg;

    always @ (posedge clk, posedge reset) begin
        if (reset)
            shift_reg <= 'b0;
        else begin
            shift_reg[16 + WIDTH - 2:0] <= shift_reg[16 + WIDTH - 1:1];
            shift_reg[16 + WIDTH - 1] <= (
                shift_reg[WIDTH + 5] ~^
                shift_reg[WIDTH + 3] ~^
                shift_reg[WIDTH + 2] ~^
                shift_reg[WIDTH]
            );
        end
    end

    assign bits = shift_reg[WIDTH-1:0];
endmodule

module tb();
    reg clk, reset;
    wire [7:0] out;

    lfsr i(clk, reset, out);

    always #1 clk = !clk;
    initial begin
        reset = 0;
        #1 reset = 1;
        #1 reset = 0;

    end
endmodule

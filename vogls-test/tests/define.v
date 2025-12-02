`define X "Hello X"
`define Y "Hello Y"

module x();
    initial begin
        $display(`X);
        $display(`Y);
    end
endmodule
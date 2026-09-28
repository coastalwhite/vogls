module top_module (
    input too_cold,
    input too_hot,
    input mode,
    input fan_on,
    output heater,
    output aircon,
    output fan
);
    assign heater = mode & too_cold;
    assign aircon = ~mode & too_hot;
    assign fan = heater | aircon | fan_on;
endmodule

module tb();
    reg a, b, c, d;
    wire x, y, z;

    top_module i( a, b, c, d, x, y, z );

    initial begin
        a=0;b=0;c=0;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b000);
        a=0;b=0;c=0;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b001);
        a=0;b=0;c=1;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b000);
        a=0;b=0;c=1;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b001);
        a=0;b=1;c=0;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b011);
        a=0;b=1;c=0;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b011);
        a=0;b=1;c=1;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b000);
        a=0;b=1;c=1;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b001);
        a=1;b=0;c=0;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b000);
        a=1;b=0;c=0;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b001);
        a=1;b=0;c=1;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b101);
        a=1;b=0;c=1;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b101);
        a=1;b=1;c=0;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b011);
        a=1;b=1;c=0;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b011);
        a=1;b=1;c=1;d=0; #1 $vogls_assert_eq({x, y, z}, 3'b101);
        a=1;b=1;c=1;d=1; #1 $vogls_assert_eq({x, y, z}, 3'b101);
    end
endmodule

// vogls: mode=two-value-logic
//
// In two-valued logic, the drivers of a `wand` resolve to their `&` and those of a `wor` to their
// `|`.
`timescale 1ns/1ns
module tb();
    reg a, b;
    reg [3:0] v, u;

    wand wa;
    wor wo;
    assign wa = a;
    assign wa = b;
    assign wo = a;
    assign wo = b;

    wand [3:0] wa_vec;
    wor [3:0] wo_vec;
    assign wa_vec = v;
    assign wa_vec[2:1] = u[1:0];
    assign wa_vec = u;
    assign wo_vec = v;
    assign wo_vec[2:1] = u[1:0];
    assign wo_vec = u;

    integer i;
    initial begin
        for (i = 0; i < 4; i = i + 1) begin
            {a, b} = i;
            #1;
            $vogls_assert_eq(wa, a & b);
            $vogls_assert_eq(wo, a | b);
        end

        v = 4'b1010;
        u = 4'b0110;
        #1;
        $vogls_assert_eq(wa_vec, 4'b0000);
        $vogls_assert_eq(wo_vec, 4'b1110);

        v = 4'b1111;
        u = 4'b1101;
        #1;
        $vogls_assert_eq(wa_vec, 4'b1001);
        $vogls_assert_eq(wo_vec, 4'b1111);
    end
endmodule

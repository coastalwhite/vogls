module y
#( parameter integer N = 4 )
(output [3:0] out);
    assign out = N;
endmodule

module z();
    wire [3:0] od;
    wire [3:0] o0;
    wire [3:0] o1;
    wire [3:0] o2;
    wire [3:0] o3;
    wire [3:0] o4;
    wire [3:0] o5;
    wire [3:0] o15;

    y           yd  (od );
    y #(.N(0))  y0  (o0 );
    y #(.N(1))  y1  (o1 );
    y #(.N(2))  y2  (o2 );
    y #(.N(3))  y3  (o3 );
    y #(.N(4))  y4  (o4 );
    y #(.N(5))  y5  (o5 );
    y #(.N(15)) y15 (o15);

    initial begin
        #1
        $vogls_assert_eq(od, 4'b0100);
        $vogls_assert_eq(o0, 4'b0000);
        $vogls_assert_eq(o1, 4'b0001);
        $vogls_assert_eq(o2, 4'b0010);
        $vogls_assert_eq(o3, 4'b0011);
        $vogls_assert_eq(o4, 4'b0100);
        $vogls_assert_eq(o5, 4'b0101);
        $vogls_assert_eq(o15, 4'b1111);
    end
endmodule

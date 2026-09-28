module tb;
    wire [3:0] bin, ok;
 
    function [3:0] gray2bin;
        input [3:0] gray;
        integer k;
        begin
            gray2bin[3] = gray[3];
            for (k = 2; k >= 0; k = k - 1)
                gray2bin[k] = gray2bin[k + 1] ^ gray[k];
        end
    endfunction
 
    assign bin = gray2bin(4'b1100);
    assign ok  = gray2bin(4'b1111);
 
    initial begin
      $vogls_assert_eq(4'b0000, gray2bin(4'b0000));
      $vogls_assert_eq(4'b0001, gray2bin(4'b0001));
      $vogls_assert_eq(4'b0010, gray2bin(4'b0011));
      $vogls_assert_eq(4'b0100, gray2bin(4'b0110));
      $vogls_assert_eq(4'b1000, gray2bin(4'b1100));
      $vogls_assert_eq(4'b1111, gray2bin(4'b1000));
      $vogls_assert_eq(4'b1010, gray2bin(4'b1111));
      $vogls_assert_eq(4'b1100, gray2bin(4'b1010));
    end

    integer i;
    reg [3:0] g;
    initial begin
        for (i = 0; i < 16; i = i + 1) begin
            g = i;
            $vogls_assert_eq(g, gray2bin(g ^ (g >> 1)));
        end
    end

    initial begin
      #1;
      $vogls_assert_eq(4'b1000, bin);
      $vogls_assert_eq(4'b1010, ok);
   end
endmodule

public class JavaTestOne {

    public static void main(String[] args) {
        testAddition();
        testLoop();
        testConditional();
        testMethod();
        System.out.println("Hello world");
    }

    public static void testAddition() {
        int a = 5;
        int b = 7;
        int result = a + b;
    }

    public static void testLoop() {
        int sum = 0;
        for (int i = 1; i <= 5; i++) {
            sum += i;
        }
    }

    public static void testConditional() {
        int x = 10;
        int y = 15;
        int result;
        
        if (x > y) {
            result = x;
        } else if (y > x) {
            result = y;
        } else {
            result = x + y;
        }
    }


    public static void testMethod() {
        int result = multiply(3, 4);
    }

    public static int multiply(int a, int b) {
        return a * b;
    }
}

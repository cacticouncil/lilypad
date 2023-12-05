public abstract class JavaBadFormatting {
    public abstract void test();
}

public interface TestInterface {
    boolean test1();
}

public class JavaBadFormatting extends Object implements TestInterface {
    private int x, y;

    public static void main(String[] args) {
        for(int i = 0; i < 10; i++)
        {
            System.out.println("This is an example of " + i);
        }
    }

        public boolean test1() {
        return false;
    }

    public void test2() {
return;
    }

    public void test3()
{
    return;
    }
}

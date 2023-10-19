using System;

public class CSharpTestOne
{
    public static void Main(string[] args)
    {
        TestAddition();
        TestLoop();
        TestConditional();
        TestMethod();
        TestCatch();
        Console.WriteLine("Hello world");
    }

    public static void TestAddition()
    {
        int a = 5;
        int b = 7;
        int result = a + b;
    }

    public static void TestLoop()
    {
        int sum = 0;
        for (int i = 1; i <= 5; i++)
        {
            sum += i;
        }
        sum = 0;
        while(sum <=5)
        {
            sum++;
        }
    }

    public static void TestConditional()
    {
        int x = 10;
        int y = 15;
        int result;

        if (x > y)
        {
            result = x;
        }
        else if (y > x)
        {
            result = y;
        }
        else
        {
            result = x + y;
        }
    }

    public static void TestMethod()
    {
        int result = Multiply(3, 4);
    }

    public static int Multiply(int a, int b)
    {
        return a * b;
    }

    public static void TestCatch(){
        try
        {
            //code
        }
        catch(Exception ex)
        {
            Console.WriteLine("An error occurred: " + ex.Message);
        }
    }

    public static void TestSwitch(){
        switch (day)
        {
        case "Monday":
            Console.WriteLine("It's Monday!");
            break;
        case "Tuesday":
            Console.WriteLine("It's Tuesday!");
            break;
        default:
            Console.WriteLine("It's some other day.");
            break;
        }
    }
}
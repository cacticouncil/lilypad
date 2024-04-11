#include <string>
#include <iostream>

using namespace std;

class MyClass
{
public:
    int myNum;
    string myString;
};

int main()
{
    MyClass myObj;

    myObj.myNum = 15;
    myObj.myString = "Some text";

    cout << myObj.myNum << "\n";
    cout << myObj.myString;

    int a = 0;
    int b = 8;

    if (a == b)
    {
        return true;
    }
    else if (a != b)
    {
        return false;
    }
    else
    {
        return 0;
    }

    for (int i = 0; i < 5; i++)
    {
        cout << i << endl;
    }
    return 0;
}
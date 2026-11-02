using System.Globalization;

namespace NamedArgs;

internal class Person
{
    private readonly string name;
    private readonly int age;

    public Person(string? name = null, int? age = null)
    {
        string name2 = name ?? "Alice";
        int age2 = age ?? 40;
        this.name = name2;
        this.age = age2;
    }

    public string Name
    {
        get
        {
            return this.name;
        }
    }

    public int Age
    {
        get
        {
            return this.age;
        }
    }

    public override string ToString()
    {
        return "{ class: Person, name: " + this.name + ", age: " + this.age.ToString(CultureInfo.InvariantCulture) + " }";
    }

    public void Greet(string? greeting = null)
    {
        string greeting2 = greeting ?? "Hi";
        NamedArgsGlobal.console.Log(greeting2 + ", " + this.name);
    }
}

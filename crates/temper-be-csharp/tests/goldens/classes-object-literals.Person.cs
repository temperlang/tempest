using ObjectLiterals.ExtraClass;
using System.Globalization;
using TemperLang.Core;

namespace ObjectLiterals;

public class Person : IStringable
{
    private readonly string name;
    private readonly int age;

    public Person(string name, int? age = null)
    {
        int age2 = age ?? -100;
        this.name = name;
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
        string ageText = Listed.Join(Listed.CreateReadOnlyList<int>(0), "", (int unused) => this.age.ToString(CultureInfo.InvariantCulture));
        return "Person " + this.name + " of age " + ageText;
    }

    public bool Eq(Person other)
    {
        if (this.name == other.name)
        {
            return this.age == other.age;
        }
        else
        {
            return false;
        }
    }
}

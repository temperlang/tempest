using ObjectLiterals.ExtraClass;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using TemperLang.Core;

namespace ObjectLiterals;

public static class ObjectLiteralsGlobal
{
    internal static readonly ILoggingConsole console;

    internal static void Bother()
    {
        Moreover whatIAm = new Moreover(1);
        Moreover moreover = new Moreover(2, 3);
        console.Log("Moreover: " + moreover.ToString());
        Moreover moreover2 = new Moreover(4);
        console.Log("Moreover again: " + moreover2.ToString());
        Moreover moreover3 = new Moreover(null, 7);
        console.Log("Moreover yet again: " + moreover3.ToString());
        Whatever whatever = new Whatever(5);
        console.Log("Whatever: " + whatever.Blah);
    }

    static ObjectLiteralsGlobal()
    {
        console = Support.Logging.LoggingConsoleFactory.CreateConsole("ObjectLiterals");
        Person person1 = new Person("Alice", 50);
        Person person2 = new Person("Bob", 45);
        Circle circle1 = new Circle(2);
        Circle circle2 = new Circle(1);
        IReadOnlyList<IStringable> things = Listed.CreateReadOnlyList<IStringable>(person1, person2, circle1, circle2);
        int fn(IStringable thing)
        {
            console.Log(thing.ToString());
            return 0;
        }
        Listed.ToReadOnlyList(Enumerable.Select(things, (Func<IStringable, int>)fn));
        _ = new Whatever(456);
        _ = new Whatever(321);
        _ = new Rectangle(7, 8);
        Bother();
        Whatever whatever = new Whatever(6);
        console.Log("Whatever again: " + whatever.Blah);
        Person personAgain = new Person("Carrie", 40);
        string name = personAgain.Name;
        int someAge = personAgain.Age;
        console.Log("name is " + name + " and age is " + someAge.ToString(CultureInfo.InvariantCulture));
        Person personMore = new Person(name, 35);
        console.Log(personMore.ToString());
        Person twin = new Person(person1.Name, person1.Age);
        console.Log("Some are twins: " + (person1.Eq(twin) ? "true" : "false"));
        console.Log("All are twins: " + (person2.Eq(twin) ? "true" : "false"));
    }
}

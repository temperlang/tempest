namespace Defaults;

internal class Greeter
{
    private readonly string greeting;
    private readonly string punct;

    public Greeter(string? greeting = null, string? punct = null)
    {
        string greeting2 = greeting ?? "Hello";
        string punct2 = punct ?? "!";
        this.greeting = greeting2;
        this.punct = punct2;
    }

    public string Greeting
    {
        get
        {
            return this.greeting;
        }
    }

    public string Punct
    {
        get
        {
            return this.punct;
        }
    }

    public string Greet(string? name = null, int? times = null)
    {
        string name2 = name ?? "world";
        int times2 = times ?? 1;
        string @out = "";
        {
            int i = 0;
            while (i < times2)
            {
                @out = @out + this.greeting + ", " + name2 + this.punct;
                i = i + 1;
            }
        }
        return @out;
    }

    public static string Shout(string text, string? mark = null)
    {
        string mark2 = mark ?? "!";
        return text + mark2 + mark2;
    }
}

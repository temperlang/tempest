namespace StaticProperties;

internal class Simple
{
    private readonly string name;

    public Simple(string name)
    {
        this.name = name;
    }

    public string Name
    {
        get
        {
            return this.name;
        }
    }

    public static Simple Simon
    {
        get
        {
            return StaticPropertiesGlobal.simpleSimon;
        }
    }

    public static string Value
    {
        get
        {
            return StaticPropertiesGlobal.simpleValue;
        }
    }

    public static int Other
    {
        get
        {
            return StaticPropertiesGlobal.simpleOther;
        }
    }
}

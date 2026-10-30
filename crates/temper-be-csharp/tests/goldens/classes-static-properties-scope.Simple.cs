namespace StaticPropertiesScope;

internal class Simple
{
    public Simple()
    {
    }

    public static string Value
    {
        get
        {
            return StaticPropertiesScopeGlobal.simpleValue;
        }
    }

    public static string Other
    {
        get
        {
            return StaticPropertiesScopeGlobal.simpleOther;
        }
    }
}

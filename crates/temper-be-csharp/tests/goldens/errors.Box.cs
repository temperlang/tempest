using TemperLang.Core;

namespace Errors;

internal class Box
{
    private readonly int value;

    public Box(int value)
    {
        this.value = value;
    }

    public int Value
    {
        get
        {
            return this.value;
        }
    }

    public int Get()
    {
        return this.value;
    }

    public string Bad()
    {
        return Core.Bubble<string>();
    }
}

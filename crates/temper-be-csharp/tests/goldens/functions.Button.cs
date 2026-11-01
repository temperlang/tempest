using System;
using System.Globalization;

namespace Functions;

internal class Button
{
    private readonly string label;
    private readonly Func<int> onPress;

    public Button(string label, Func<int> onPress)
    {
        this.label = label;
        this.onPress = onPress;
    }

    public string Label
    {
        get
        {
            return this.label;
        }
    }

    public Func<int> OnPress
    {
        get
        {
            return this.onPress;
        }
    }

    public string Press()
    {
        return this.label + " " + this.onPress().ToString(CultureInfo.InvariantCulture);
    }
}

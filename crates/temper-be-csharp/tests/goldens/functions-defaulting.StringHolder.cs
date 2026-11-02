namespace Defaulting;

internal class StringHolder
{
    private readonly string @string;

    public StringHolder(string @string)
    {
        this.@string = @string;
    }

    public string String
    {
        get
        {
            return this.@string;
        }
    }
}

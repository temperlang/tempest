using System.Text;

namespace Build;

internal class StringBuilderOwner
{
    private readonly StringBuilder builder;

    public StringBuilderOwner(StringBuilder builder)
    {
        this.builder = builder;
    }

    public StringBuilder Builder
    {
        get
        {
            return this.builder;
        }
    }
}

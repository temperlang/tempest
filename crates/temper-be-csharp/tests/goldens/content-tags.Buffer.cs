using System.Text;

namespace ContentTags;

public class Buffer
{
    private readonly StringBuilder sb;

    public Buffer()
    {
        this.sb = new StringBuilder();
    }

    public void Append(string x)
    {
        this.sb.Append(x);
    }

    public void Mark(Mark m)
    {
        this.sb.Append("{" + m.Label + "}");
    }

    public override string ToString()
    {
        return this.sb.ToString();
    }
}

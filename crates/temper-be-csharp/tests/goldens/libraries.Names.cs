namespace Libraries;

public class Names : ISink<Tag>
{
    public Names()
    {
    }

    public int Take(Tag e)
    {
        if (e.Name == "abc")
        {
            return 3;
        }
        else
        {
            return 0;
        }
    }
}

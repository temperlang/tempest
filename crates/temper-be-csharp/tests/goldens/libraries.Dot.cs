namespace Libraries;

public class Dot : IShape
{
    private readonly int size;

    public Dot(int size)
    {
        this.size = size;
    }

    public int Size
    {
        get
        {
            return this.size;
        }
    }

    public Dot? Bigger
    {
        get
        {
            if (this.size < 3)
            {
                return new Dot(this.size + 1);
            }
            else
            {
                return null;
            }
        }
    }

    public int? Grown
    {
        get
        {
            {
                Dot? subject = this.Bigger;
                if (subject == null)
                {
                    return null;
                }
                else
                {
                    return subject.Scaled(10);
                }
            }
        }
    }

    public int Scaled(int k)
    {
        return this.size * k;
    }
}

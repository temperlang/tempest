using Microsoft.VisualStudio.TestTools.UnitTesting;
using TemperLang.Std.Testing;

namespace Testing;

[TestClass]
public class TestingTests
{
    [TestMethod]
    public void TwiceWorks()
    {
        Test test = new Test();
        try
        {
            int x = TestingGlobal.Twice(2);
            TestingGlobal.runs = TestingGlobal.runs + x + TestingGlobal.@base;
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void NamesPunctuationQuotesAndÜnïcode()
    {
        Test t = new Test();
        try
        {
            TestingGlobal.Twice(TestingGlobal.runs);
        }
        finally
        {
            t.SoftFailToHard();
        }
    }

    [TestMethod]
    public void LocalsATestAssigns()
    {
        Test test = new Test();
        try
        {
            int n = 0;
            {
                int i = 0;
                while (i < 3)
                {
                    n = n + TestingGlobal.Twice(i);
                    i = i + 1;
                }
            }
            TestingGlobal.runs = n;
        }
        finally
        {
            test.SoftFailToHard();
        }
    }
}

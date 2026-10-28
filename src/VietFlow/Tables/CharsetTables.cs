// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

namespace VietFlow.Tables;

using System.Runtime.CompilerServices;
using VietFlow.Engine;

public static partial class CharsetTables
{
    public const int TotalVnChars = 213;
    public const int TotalAlphaVnChars = 186;
    public const uint VnStdCharOffset = 0x10000;
    public const uint InvalidStdChar = 0xFFFFFFFF;

    public static readonly char[] UnicodeTable =
    [
        '\u0041', '\u0061', '\u00c1', '\u00e1', '\u00c0', '\u00e0', '\u1ea2', '\u1ea3', '\u00c3', '\u00e3', '\u1ea0', '\u1ea1', // a
        '\u00c2', '\u00e2', '\u1ea4', '\u1ea5', '\u1ea6', '\u1ea7', '\u1ea8', '\u1ea9', '\u1eaa', '\u1eab', '\u1eac', '\u1ead', // a^
        '\u0102', '\u0103', '\u1eae', '\u1eaf', '\u1eb0', '\u1eb1', '\u1eb2', '\u1eb3', '\u1eb4', '\u1eb5', '\u1eb6', '\u1eb7', // a(
        '\u0042', '\u0062', '\u0043', '\u0063', '\u0044', '\u0064',                                                                 // B b C c D d
        '\u0110', '\u0111',                                                                                                         // DD, dd
        '\u0045', '\u0065', '\u00c9', '\u00e9', '\u00c8', '\u00e8', '\u1eba', '\u1ebb', '\u1ebc', '\u1ebd', '\u1eb8', '\u1eb9', // e
        '\u00ca', '\u00ea', '\u1ebe', '\u1ebf', '\u1ec0', '\u1ec1', '\u1ec2', '\u1ec3', '\u1ec4', '\u1ec5', '\u1ec6', '\u1ec7', // e^
        '\u0046', '\u0066', '\u0047', '\u0067', '\u0048', '\u0068',                                                                 // F f G g H h
        '\u0049', '\u0069', '\u00cd', '\u00ed', '\u00cc', '\u00ec', '\u1ec8', '\u1ec9', '\u0128', '\u0129', '\u1eca', '\u1ecb', // i
        '\u004a', '\u006a', '\u004b', '\u006b', '\u004c', '\u006c', '\u004d', '\u006d', '\u004e', '\u006e',                         // J j K k L l M m N n
        '\u004f', '\u006f', '\u00d3', '\u00f3', '\u00d2', '\u00f2', '\u1ece', '\u1ecf', '\u00d5', '\u00f5', '\u1ecc', '\u1ecd', // o
        '\u00d4', '\u00f4', '\u1ed0', '\u1ed1', '\u1ed2', '\u1ed3', '\u1ed4', '\u1ed5', '\u1ed6', '\u1ed7', '\u1ed8', '\u1ed9', // o^
        '\u01a0', '\u01a1', '\u1eda', '\u1edb', '\u1edc', '\u1edd', '\u1ede', '\u1edf', '\u1ee0', '\u1ee1', '\u1ee2', '\u1ee3', // o+
        '\u0050', '\u0070', '\u0051', '\u0071', '\u0052', '\u0072', '\u0053', '\u0073', '\u0054', '\u0074',                         // P p Q q R r S s T t
        '\u0055', '\u0075', '\u00da', '\u00fa', '\u00d9', '\u00f9', '\u1ee6', '\u1ee7', '\u0168', '\u0169', '\u1ee4', '\u1ee5', // u
        '\u01af', '\u01b0', '\u1ee8', '\u1ee9', '\u1eea', '\u1eeb', '\u1eec', '\u1eed', '\u1eee', '\u1eef', '\u1ef0', '\u1ef1', // u+
        '\u0056', '\u0076', '\u0057', '\u0077', '\u0058', '\u0078',                                                                 // V v W w X x
        '\u0059', '\u0079', '\u00dd', '\u00fd', '\u1ef2', '\u1ef3', '\u1ef6', '\u1ef7', '\u1ef8', '\u1ef9', '\u1ef4', '\u1ef5', // y
        '\u005a', '\u007a',                                                                                                         // Z z
        // Symbols that have different code points in Unicode and Western charsets
        '\u20AC', '\u20A1', '\u0192', '\u201E', '\u2026', '\u2020', '\u2021', '\u02C6',
        '\u2030', '\u0160', '\u2039', '\u0152', '\u017D', '\u2018', '\u2019', '\u201C',
        '\u201D', '\u2022', '\u2013', '\u2014', '\u02DC', '\u2122', '\u0161', '\u203A',
        '\u0153', '\u017E', '\u0178'
    ];

    public static readonly uint[] UnicodeComposite =
    [
        0x00000041, 0x00000061, 0x03010041, 0x03010061, 0x03000041, 0x03000061,
        0x03090041, 0x03090061, 0x03030041, 0x03030061, 0x03230041, 0x03230061, // a

        0x000000c2, 0x000000e2, 0x030100c2, 0x030100e2, 0x030000c2, 0x030000e2,
        0x030900c2, 0x030900e2, 0x030300c2, 0x030300e2, 0x032300c2, 0x032300e2, // a^

        0x00000102, 0x00000103, 0x03010102, 0x03010103, 0x03000102, 0x03000103,
        0x03090102, 0x03090103, 0x03030102, 0x03030103, 0x03230102, 0x03230103, // a(

        0x0042, 0x0062, 0x0043, 0x0063, 0x0044, 0x0064,                         // B b C c D d
        0x0110, 0x0111,                                                         // DD, dd

        0x00000045, 0x00000065, 0x03010045, 0x03010065, 0x03000045, 0x03000065,
        0x03090045, 0x03090065, 0x03030045, 0x03030065, 0x03230045, 0x03230065, // e

        0x000000ca, 0x000000ea, 0x030100ca, 0x030100ea, 0x030000ca, 0x030000ea,
        0x030900ca, 0x030900ea, 0x030300ca, 0x030300ea, 0x032300ca, 0x032300ea, // e^

        0x0046, 0x0066, 0x0047, 0x0067, 0x0048, 0x0068,                         // F f G g H h

        0x00000049, 0x00000069, 0x03010049, 0x03010069, 0x03000049, 0x03000069,
        0x03090049, 0x03090069, 0x03030049, 0x03030069, 0x03230049, 0x03230069, // i

        0x004a, 0x006a, 0x004b, 0x006b, 0x004c, 0x006c, 0x004d, 0x006d, 0x004e, 0x006e, // J j K k L l M m N n

        0x0000004f, 0x0000006f, 0x0301004f, 0x0301006f, 0x0300004f, 0x0300006f,
        0x0309004f, 0x0309006f, 0x0303004f, 0x0303006f, 0x0323004f, 0x0323006f, // o

        0x000000d4, 0x000000f4, 0x030100d4, 0x030100f4, 0x030000d4, 0x030000f4,
        0x030900d4, 0x030900f4, 0x030300d4, 0x030300f4, 0x032300d4, 0x032300f4, // o^

        0x000001a0, 0x000001a1, 0x030101a0, 0x030101a1, 0x030001a0, 0x030001a1,
        0x030901a0, 0x030901a1, 0x030301a0, 0x030301a1, 0x032301a0, 0x032301a1, // o+

        0x0050, 0x0070, 0x0051, 0x0071, 0x0052, 0x0072, 0x0053, 0x0073, 0x0054, 0x0074, // P p Q q R r S s T t

        0x00000055, 0x00000075, 0x03010055, 0x03010075, 0x03000055, 0x03000075,
        0x03090055, 0x03090075, 0x03030055, 0x03030075, 0x03230055, 0x03230075, // u

        0x000001af, 0x000001b0, 0x030101af, 0x030101b0, 0x030001af, 0x030001b0,
        0x030901af, 0x030901b0, 0x030301af, 0x030301b0, 0x032301af, 0x032301b0, // u+

        0x0056, 0x0076, 0x0057, 0x0077, 0x0058, 0x0078,                         // V v W w X x

        0x00000059, 0x00000079, 0x03010059, 0x03010079, 0x03000059, 0x03000079,
        0x03090059, 0x03090079, 0x03030059, 0x03030079, 0x03230059, 0x03230079, // y
        0x005a, 0x007a,                                                         // Z z
        // Symbols that have different code points in Unicode and Western charsets
        0x20AC, 0x20A1, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6,
        0x2030, 0x0160, 0x2039, 0x0152, 0x017D, 0x2018, 0x2019, 0x201C,
        0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A,
        0x0153, 0x017E, 0x0178
    ];

    public static readonly int[] StdVnRootChar =
    [
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, // a [A=0]
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, // a^ -> a
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, // a( -> a
        36, 37, 38, 39, 40, 41,             // bcd [D=40, d=41]
        40, 41,                             // DD dd [mapped to D, d]
        44, 45, 44, 45, 44, 45, 44, 45, 44, 45, 44, 45, // e [E = 44]
        44, 45, 44, 45, 44, 45, 44, 45, 44, 45, 44, 45, // e^ -> e
        68, 69, 70, 71, 72, 73,             // fgh
        74, 75, 74, 75, 74, 75, 74, 75, 74, 75, 74, 75, // i
        86, 87, 88, 89, 90, 91, 92, 93, 94, 95,          // jklmn
        96, 97, 96, 97, 96, 97, 96, 97, 96, 97, 96, 97, // o [o=96]
        96, 97, 96, 97, 96, 97, 96, 97, 96, 97, 96, 97, // o^ -> o
        96, 97, 96, 97, 96, 97, 96, 97, 96, 97, 96, 97, // o+ -> o
        132, 133, 134, 135, 136, 137, 138, 139, 140, 141, // pqrst
        142, 143, 142, 143, 142, 143, 142, 143, 142, 143, 142, 143, // u [U=142]
        142, 143, 142, 143, 142, 143, 142, 143, 142, 143, 142, 143, // u+ -> u
        166, 167, 168, 169, 170, 171, // vwx
        172, 173, 172, 173, 172, 173, 172, 173, 172, 173, 172, 173, // y [Y=172]
        184, 185, // z
        186, 187, 188, 189, 190, 191, 192, 193,
        194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209,
        210, 211, 212
    ];

    public static readonly int[] StdVnNoTone =
    [
        0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, // a [A=0]
        12, 13, 12, 13, 12, 13, 12, 13, 12, 13, 12, 13, // a^
        24, 25, 24, 25, 24, 25, 24, 25, 24, 25, 24, 25, // a(
        36, 37, 38, 39, 40, 41, // bcd [D=40, d=41]
        42, 43,                 // DD dd
        44, 45, 44, 45, 44, 45, 44, 45, 44, 45, 44, 45, // e [E = 44]
        56, 57, 56, 57, 56, 57, 56, 57, 56, 57, 56, 57, // e^
        68, 69, 70, 71, 72, 73, // fgh
        74, 75, 74, 75, 74, 75, 74, 75, 74, 75, 74, 75, // i
        86, 87, 88, 89, 90, 91, 92, 93, 94, 95,          // jklmn
        96, 97, 96, 97, 96, 97, 96, 97, 96, 97, 96, 97, // o [o=96]
        108, 109, 108, 109, 108, 109, 108, 109, 108, 109, 108, 109, // o^
        120, 121, 120, 121, 120, 121, 120, 121, 120, 121, 120, 121, // o+
        132, 133, 134, 135, 136, 137, 138, 139, 140, 141, // pqrst
        142, 143, 142, 143, 142, 143, 142, 143, 142, 143, 142, 143, // u [U=142]
        154, 155, 154, 155, 154, 155, 154, 155, 154, 155, 154, 155, // u+
        166, 167, 168, 169, 170, 171, // vwx
        172, 173, 172, 173, 172, 173, 172, 173, 172, 173, 172, 173, // y [Y=172]
        184, 185, // z
        186, 187, 188, 189, 190, 191, 192, 193,
        194, 195, 196, 197, 198, 199, 200, 201,
        202, 203, 204, 205, 206, 207, 208, 209,
        210, 211, 212
    ];

    public static readonly byte[] SpecialWesternChars =
    [
        0x80, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
        0x89, 0x8A, 0x8B, 0x8C, 0x8E, 0x91, 0x92, 0x93,
        0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B,
        0x9C, 0x9E, 0x9F
    ];

    public static readonly (int Asc, VnLexiName Lexi)[] AscVnLexiList =
    [
        (0xC0, VnLexiName.A2), (0xC1, VnLexiName.A1), (0xC2, VnLexiName.Ar), (0xC2, VnLexiName.A4),
        (0xC8, VnLexiName.E2), (0xC9, VnLexiName.E1), (0xCA, VnLexiName.Er),
        (0xCC, VnLexiName.I2), (0xCD, VnLexiName.I1),
        (0xD2, VnLexiName.O2), (0xD3, VnLexiName.O1), (0xD4, VnLexiName.Or), (0xD5, VnLexiName.O4),
        (0xD9, VnLexiName.U2), (0xDA, VnLexiName.U1),
        (0xDD, VnLexiName.Y1),
        (0xE0, VnLexiName.a2), (0xE1, VnLexiName.a1), (0xE2, VnLexiName.ar), (0xE3, VnLexiName.a4),
        (0xE8, VnLexiName.e2), (0xE9, VnLexiName.e1), (0xEA, VnLexiName.er),
        (0xEC, VnLexiName.i2), (0xED, VnLexiName.i1),
        (0xF2, VnLexiName.o2), (0xF3, VnLexiName.o1), (0xF4, VnLexiName.or), (0xF5, VnLexiName.o4),
        (0xF9, VnLexiName.u2), (0xFA, VnLexiName.u1),
        (0xFD, VnLexiName.y1)
    ];

    public static readonly VnLexiName[] IsoVnLexiMap = new VnLexiName[256];
    public static readonly uint[] IsoStdVnCharMap = new uint[256];
    public static readonly bool[] IsVnVowel = new bool[TotalVnChars];
    public static readonly char[] VnLexiToIso = new char[TotalVnChars];

    static CharsetTables()
    {
        Array.Fill(IsoVnLexiMap, VnLexiName.NonVnChar);
        foreach (var item in AscVnLexiList)
        {
            IsoVnLexiMap[item.Asc] = item.Lexi;
        }
        for (int c = 'a'; c <= 'z'; c++)
        {
            IsoVnLexiMap[c] = (VnLexiName)AZLexiLower[c - 'a'];
        }
        for (int c = 'A'; c <= 'Z'; c++)
        {
            IsoVnLexiMap[c] = (VnLexiName)AZLexiUpper[c - 'A'];
        }

        for (uint i = 0; i < 256; i++)
        {
            IsoStdVnCharMap[i] = i;
        }
        for (int i = 0; i < SpecialWesternChars.Length; i++)
        {
            IsoStdVnCharMap[SpecialWesternChars[i]] = (uint)((int)VnLexiName.LastChar + i) + VnStdCharOffset;
        }
        for (int i = 0; i < 256; i++)
        {
            var lexi = IsoVnLexiMap[i];
            if (lexi != VnLexiName.NonVnChar)
            {
                IsoStdVnCharMap[i] = (uint)lexi + VnStdCharOffset;
            }
        }

        Array.Fill(IsVnVowel, true);
        for (char ch = 'a'; ch <= 'z'; ch++)
        {
            if (ch != 'a' && ch != 'e' && ch != 'i' && ch != 'o' && ch != 'u' && ch != 'y')
            {
                IsVnVowel[(int)AZLexiLower[ch - 'a']] = false;
                IsVnVowel[(int)AZLexiUpper[ch - 'a']] = false;
            }
        }
        IsVnVowel[(int)VnLexiName.dd] = false;
        IsVnVowel[(int)VnLexiName.DD] = false;

        // Reverse map (VnLexiName -> ASCII letter) used for consonant validation. 0 means "not a letter".
        for (char ch = 'a'; ch <= 'z'; ch++)
        {
            VnLexiToIso[(int)AZLexiLower[ch - 'a']] = ch;
        }
        for (char ch = 'A'; ch <= 'Z'; ch++)
        {
            VnLexiToIso[(int)AZLexiUpper[ch - 'A']] = ch;
        }
        VnLexiToIso[(int)VnLexiName.dd] = 'd';
        VnLexiToIso[(int)VnLexiName.DD] = 'D';
    }

    private static readonly VnLexiName[] AZLexiUpper =
    [
        VnLexiName.A, VnLexiName.B, VnLexiName.C, VnLexiName.D, VnLexiName.E,
        VnLexiName.F, VnLexiName.G, VnLexiName.H, VnLexiName.I, VnLexiName.J,
        VnLexiName.K, VnLexiName.L, VnLexiName.M, VnLexiName.N, VnLexiName.O,
        VnLexiName.P, VnLexiName.Q, VnLexiName.R, VnLexiName.S, VnLexiName.T,
        VnLexiName.U, VnLexiName.V, VnLexiName.W, VnLexiName.X, VnLexiName.Y, VnLexiName.Z
    ];

    private static readonly VnLexiName[] AZLexiLower =
    [
        VnLexiName.a, VnLexiName.b, VnLexiName.c, VnLexiName.d, VnLexiName.e,
        VnLexiName.f, VnLexiName.g, VnLexiName.h, VnLexiName.i, VnLexiName.j,
        VnLexiName.k, VnLexiName.l, VnLexiName.m, VnLexiName.n, VnLexiName.o,
        VnLexiName.p, VnLexiName.q, VnLexiName.r, VnLexiName.s, VnLexiName.t,
        VnLexiName.u, VnLexiName.v, VnLexiName.w, VnLexiName.x, VnLexiName.y, VnLexiName.z
    ];

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static VnLexiName IsoToVnLexi(uint keyCode) =>
        keyCode < 256 ? IsoVnLexiMap[keyCode] : VnLexiName.NonVnChar;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static uint IsoToStdVnChar(uint keyCode) =>
        keyCode < 256 ? IsoStdVnCharMap[keyCode] : keyCode;
}

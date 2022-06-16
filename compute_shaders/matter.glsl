struct Matter {
    uint matter;
    uint color;
};

Matter new_matter(uint matter) {
    Matter m;
    m.matter = (matter & uint(255));
    m.color = matter >> uint(8);
    return m;
}

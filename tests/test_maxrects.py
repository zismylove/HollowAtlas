from hollowatlas.core.maxrects import MaxRectsPacker


def test_maxrects_places_without_overlap():
    packer = MaxRectsPacker(64, 64)
    placed = [
        packer.insert(0, 32, 32),
        packer.insert(1, 16, 16),
        packer.insert(2, 20, 12),
        packer.insert(3, 8, 40),
    ]

    rects = [item.rect for item in placed if item is not None]
    assert len(rects) == 4
    for index, rect in enumerate(rects):
        assert rect.right <= 64
        assert rect.bottom <= 64
        for other in rects[index + 1 :]:
            assert not rect.intersects(other)

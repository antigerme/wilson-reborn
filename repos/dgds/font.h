/* ScummVM - Graphic Adventure Engine
 *
 * ScummVM is the legal property of its developers, whose names
 * are too numerous to list here. Please refer to the COPYRIGHT
 * file distributed with this source distribution.
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301, USA.
 *
 */

#ifndef DGDS_FONT_H
#define DGDS_FONT_H

#include "common/scummsys.h"

namespace Graphics {
class Font;
class Surface;
}

namespace Common {
class SeekableReadStream;
}

namespace Dgds {

class Font : public Graphics::Font {
protected:
	byte _w;
	byte _h;
	byte _start;
	byte _count;
	byte *_data;

        bool hasChar(byte chr) const;

public:
	int getFontHeight() const;
	int getMaxCharWidth() const;
	virtual int getCharWidth(uint32 chr) const = 0;
        void drawChar(Graphics::Surface* dst, int pos, int bit, int x, int y, uint32 color) const;
};

class PFont : public Font {
protected:
	uint16 *_offsets;
	byte *_widths;

	void mapChar(byte chr, int& pos, int& bit) const;

public:
	int getCharWidth(uint32 chr) const;
	void drawChar(Graphics::Surface* dst, uint32 chr, int x, int y, uint32 color) const;

	static PFont *load_PFont(Common::SeekableReadStream &input);
};

class FFont : public Font {
protected:
	void mapChar(byte chr, int& pos, int& bit) const;

public:
	int getCharWidth(uint32 chr) const;
	void drawChar(Graphics::Surface* dst, uint32 chr, int x, int y, uint32 color) const;

	static FFont *load_Font(Common::SeekableReadStream &input);
};

} // End of namespace Dgds

#endif // DGDS_FONT_H

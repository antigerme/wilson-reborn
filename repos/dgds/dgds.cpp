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

#include "common/config-manager.h"
#include "common/debug.h"
#include "common/debug-channels.h"
#include "common/endian.h"
#include "common/events.h"
#include "common/file.h"
#include "common/stream.h"
#include "common/memstream.h"
#include "common/substream.h"
#include "common/system.h"
#include "common/platform.h"
#include "common/str-array.h"

#include "common/iff_container.h"

#include "audio/audiostream.h"
#include "audio/mixer.h"
#include "audio/decoders/aiff.h"
#include "audio/decoders/raw.h"

#include "graphics/palette.h"
#include "graphics/font.h"
#include "graphics/fontman.h"
#include "graphics/managed_surface.h"
#include "graphics/surface.h"

#include "engines/advancedDetector.h"
#include "engines/util.h"

#include "dgds/decompress.h"
#include "dgds/detection_tables.h"
#include "dgds/font.h"
#include "dgds/sound.h"
#include "dgds/music.h"

#include "dgds/dgds.h"

namespace Dgds {
	typedef unsigned uint32;
	typedef unsigned short uint16;
	typedef unsigned char uint8;
	typedef unsigned char byte;

byte palette[256*3];
byte blacks[256*3];

Graphics::Surface binData;
Graphics::Surface vgaData;
Graphics::Surface ma8Data;

Graphics::Surface _binData;
Graphics::Surface _vgaData;
Graphics::Surface _bmpData;

Graphics::Surface bottomBuffer;
Graphics::Surface topBuffer;

Graphics::ManagedSurface resData;

Common::MemoryReadStream *soundData;
byte *musicData;
uint32 musicSize;

Common::StringArray BMPs;
Common::StringArray _bubbles;

uint16 _tcount;
uint16 _tw, _th;
uint32 _toffset;

uint16 *_mtx;
uint16 _mw, _mh;

uint16 _count;
char **_strs;
uint16 *_idxs;

#define DGDS_FILENAME_MAX 12
#define DGDS_TYPENAME_MAX 4

Common::SeekableReadStream* ttm;
char ttmName[DGDS_FILENAME_MAX+1];

Common::SeekableReadStream* ads;
char adsName[DGDS_FILENAME_MAX+1];

int sw = 320, sh = 200;

DgdsEngine::DgdsEngine(OSystem *syst, const DgdsGameDescription *gameDesc)
 : Engine(syst) {
	 syncSoundSettings();

	_console = new DgdsConsole(this);

	_platform = gameDesc->desc.platform;
	_rmfName = gameDesc->desc.filesDescriptions[0].fileName;
}

DgdsEngine::~DgdsEngine() {
	DebugMan.clearAllDebugChannels();

	delete _console;
}

typedef uint32 DGDS_ID;
typedef uint32 DGDS_EX;

#define MKTAG24(a0,a1,a2) ((uint32)((a2) | (a1) << 8 | ((a0) << 16)))

struct DgdsParser;

struct DgdsChunk {
	char id[DGDS_TYPENAME_MAX+1];
	DGDS_ID _id;
	uint32 _size;
	bool container;
	Common::SeekableReadStream* _stream;

	bool isSection(const Common::String& section);
	bool isSection(DGDS_ID section);
	bool isPacked(DGDS_EX ex);

	bool readHeader(DgdsParser& ctx);
	Common::SeekableReadStream* decodeStream(DgdsParser& ctx);
	Common::SeekableReadStream* readStream(DgdsParser& ctx);
};

struct DgdsParser {
	char _filename[13];
	Common::SeekableReadStream& _file;
	uint32 bytesRead;
	
	/**
	 * Callback type for the parser.
	 */
	typedef Common::Functor1< DgdsChunk&, bool > DgdsCallback;

	DgdsParser(Common::SeekableReadStream& file, const char *filename);
	void parse(DgdsCallback &callback);
};

void DgdsParser::parse(DgdsCallback &callback) {
	const char *dot;
	DGDS_EX _ex;

	if ((dot = strrchr(_filename, '.'))) {
		_ex = MKTAG24(toupper(dot[1]), toupper(dot[2]), toupper(dot[3]));
	} else {
		_ex = 0;
	}

	DgdsChunk chunk;
	while (chunk.readHeader(*this)) {
		bool stop;

		chunk._stream = 0;
		if (chunk.container) {
			stop = callback(chunk);
		} else {
			chunk._stream = chunk.isPacked(_ex) ? chunk.decodeStream(*this) : chunk.readStream(*this);

			stop = callback(chunk);

			int leftover = chunk._stream->size()-chunk._stream->pos();
			chunk._stream->skip(leftover);
			delete chunk._stream;
		}
		if (stop) break;
	}
}

DgdsParser::DgdsParser(Common::SeekableReadStream& file, const char *filename) : _file(file) {
	Common::strlcpy(_filename, filename, sizeof(_filename));
	bytesRead = 0;
}

#define ID_BIN	MKTAG24('B','I','N')
#define ID_DAT	MKTAG24('D','A','T')
#define ID_FNM	MKTAG24('F','N','M')
#define ID_FNT	MKTAG24('F','N','T')
#define ID_GAD	MKTAG24('G','A','D')
#define ID_INF	MKTAG24('I','N','F')
#define ID_MTX	MKTAG24('M','T','X')
#define ID_PAG	MKTAG24('P','A','G')
#define ID_REQ	MKTAG24('R','E','Q')
#define ID_RES	MKTAG24('R','E','S')
#define ID_SCR	MKTAG24('S','C','R')
#define ID_SDS	MKTAG24('S','D','S')
#define ID_SNG	MKTAG24('S','N','G')
#define ID_TAG	MKTAG24('T','A','G')
#define ID_TT3	MKTAG24('T','T','3')
#define ID_TTI	MKTAG24('T','T','I')
#define ID_VER	MKTAG24('V','E','R')
#define ID_VGA	MKTAG24('V','G','A')
#define ID_VQT	MKTAG24('V','Q','T')

/* Heart of China */
#define ID_MA8	MKTAG24('M','A','8')
#define ID_DDS	MKTAG24('D','D','S')
#define ID_THD	MKTAG24('T','H','D')

#define	EX_ADH	MKTAG24('A','D','H')
#define	EX_ADL	MKTAG24('A','D','L')
#define	EX_ADS	MKTAG24('A','D','S')
#define	EX_AMG	MKTAG24('A','M','G')
#define	EX_BMP	MKTAG24('B','M','P')
#define	EX_GDS	MKTAG24('G','D','S')
#define	EX_INS	MKTAG24('I','N','S')
#define	EX_PAL	MKTAG24('P','A','L')
#define	EX_FNT	MKTAG24('F','N','T')
#define	EX_REQ	MKTAG24('R','E','Q')
#define	EX_RST	MKTAG24('R','S','T')
#define	EX_SCR	MKTAG24('S','C','R')
#define	EX_SDS	MKTAG24('S','D','S')
#define	EX_SNG	MKTAG24('S','N','G')
#define	EX_SX	MKTAG24('S','X', 0 )
#define	EX_TTM	MKTAG24('T','T','M')
#define	EX_VIN	MKTAG24('V','I','N')

/* Heart of China */
#define	EX_DAT	MKTAG24('D','A','T')
#define	EX_DDS	MKTAG24('D','D','S')
#define	EX_TDS	MKTAG24('T','D','S')

#define	EX_OVL	MKTAG24('O','V','L')

bool DgdsChunk::isSection(const Common::String& section) {
       return section.equals(id);
}

bool DgdsChunk::isSection(DGDS_ID section) {
       return (section == _id);
}

bool isFlatfile(Common::Platform platform, DGDS_EX _ex) {
	bool flat = false;

	switch (_ex) {
		case EX_RST:
		case EX_VIN:
		case EX_DAT:
			flat = true;
			break;
		default:
			break;
	}

	switch (platform) {
		case Common::kPlatformAmiga:
			switch (_ex) {
				case EX_BMP:
				case EX_SCR:
				case EX_INS:
//				case EX_SNG:
				case EX_AMG:
					flat = true;
					break;
				default:
					break;
			}
		default:
			break;
	}
	return flat;
}

bool DgdsChunk::isPacked(DGDS_EX ex) {
	bool packed = false;

	switch (ex) {
		case EX_ADS:
		case EX_ADL:
		case EX_ADH:
			if (0) {}
			else if (_id == ID_SCR) packed = true;
			break;
		case EX_BMP:
			if (0) {}
			else if (_id == ID_BIN) packed = true;
			else if (_id == ID_VGA) packed = true;
			break;
		case EX_GDS:
			if (0) {}
			else if (_id == ID_SDS) packed = true;
			break;
		case EX_SCR:
			if (0) {}
			else if (_id == ID_BIN) packed = true;
			else if (_id == ID_VGA) packed = true;
			else if (_id == ID_MA8) packed = true;
			break;
		case EX_SDS:
			if (0) {}
			else if (_id == ID_SDS) packed = true;
			break;
		case EX_SNG:
			if (0) {}
			else if (_id == ID_SNG) packed = true;
			break;
		case EX_TTM:
			if (0) {}
			else if (_id == ID_TT3) packed = true;
			break;
		case EX_TDS:
			if (0) {}
			else if (_id == ID_THD) packed = true;
			break;
		default:
			break;
	}

	switch (ex) {
		case EX_DDS:
			if (0) {}
			else if (strcmp(id, "DDS:") == 0) packed = true;
			break;
		case EX_OVL:
			if (0) {}
			else if (strcmp(id, "ADL:") == 0) packed = true;
			else if (strcmp(id, "ADS:") == 0) packed = true;
			else if (strcmp(id, "APA:") == 0) packed = true;
			else if (strcmp(id, "ASB:") == 0) packed = true;
			else if (strcmp(id, "GMD:") == 0) packed = true;
			else if (strcmp(id, "M32:") == 0) packed = true;
			else if (strcmp(id, "NLD:") == 0) packed = true;
			else if (strcmp(id, "PRO:") == 0) packed = true;
			else if (strcmp(id, "PS1:") == 0) packed = true;
			else if (strcmp(id, "SBL:") == 0) packed = true;
			else if (strcmp(id, "SBP:") == 0) packed = true;
			else if (strcmp(id, "STD:") == 0) packed = true;
			else if (strcmp(id, "TAN:") == 0) packed = true;
			else if (strcmp(id, "T3V:") == 0) packed = true;
			else if (strcmp(id, "001:") == 0) packed = true;
			else if (strcmp(id, "003:") == 0) packed = true;
			else if (strcmp(id, "004:") == 0) packed = true;
			else if (strcmp(id, "101:") == 0) packed = true;
			else if (strcmp(id, "VGA:") == 0) packed = true;
			break;
		case EX_TDS:
			if (0) {}
			else if (strcmp(id, "TDS:") == 0) packed = true;  /* ? */
			break;
		default:
			break;
	}
	return packed;
}

bool DgdsChunk::readHeader(DgdsParser& ctx) {
	memset(id, 0, sizeof(id));
	_id = 0;

	if (ctx._file.pos() >= ctx._file.size()) {
		return false;
	}

	ctx._file.read(id, DGDS_TYPENAME_MAX);

	if (id[DGDS_TYPENAME_MAX-1] != ':') {
		debug("bad header in: %s", ctx._filename);
		return false;
	}
	id[DGDS_TYPENAME_MAX] = '\0';
	_id = MKTAG24(uint32(id[0]), uint32(id[1]), uint32(id[2]));

	_size = ctx._file.readUint32LE();
	if (_size & 0x80000000) {
		_size &= ~0x80000000;
		container = true;
	} else {
		container = false;
	}
	return true;
}
 
Common::SeekableReadStream* DgdsChunk::decodeStream(DgdsParser& ctx) {
	byte compression;
	uint32 unpackSize;
	Common::SeekableReadStream *output = 0;

	compression = ctx._file.readByte();
	unpackSize = ctx._file.readUint32LE();
	_size -= (1 + 4);

	if (!container) {
		byte *dest = new byte[unpackSize];
		decompress(compression, dest, unpackSize, ctx._file, _size);
		output = new Common::MemoryReadStream(dest, unpackSize, DisposeAfterUse::YES);
		ctx.bytesRead += unpackSize;
	}

	debug("    %s %u %s %u%c",
		id, _size,
		compressionDescr[compression],
		unpackSize, (container ? '+' : ' '));
	return output;
}

Common::SeekableReadStream* DgdsChunk::readStream(DgdsParser& ctx) {
	Common::SeekableReadStream *output = 0;

	if (!container) {
		output = new Common::SeekableSubReadStream(&ctx._file, ctx._file.pos(), ctx._file.pos()+_size, DisposeAfterUse::NO);
		ctx.bytesRead += _size;
	}

	debug("    %s %u%c", id, _size, (container ? '+' : ' '));
	return output;
}

/*
  input:
    s - pointer to ASCIIZ filename string
    idx - pointer to an array of 4 bytes (hash indexes)
  return:
    hash - filename hash
*/
int32 dgdsHash(const char *s, byte *idx) {
	int32 i, c;
	int16 isum, ixor;
	isum = 0;
	ixor = 0;
	for (i = 0; s[i]; i++) {
		c = toupper(s[i]);
		isum += c;
		ixor ^= c;
	}
	/* both types here MUST be int16 */
	isum *= ixor;
	c = 0;
	for (ixor = 0; ixor < 4; ixor++) {
		c <<= 8;
		/* can use only existing characters
		   ("i" holds the string length now) */
		if (i > idx[ixor]) {
			c |= toupper(s[idx[ixor]]);
		}
	}
	c += isum;
	return c;
}

uint16 readStrings(Common::SeekableReadStream* stream){
	uint16 count;
	
	count = stream->readUint16LE();
	debug("        %u:", count);
	for (uint16 k=0; k<count; k++) {
		byte ch;
		uint16 idx;
		idx = stream->readUint16LE();
		
		Common::String str;
		while ((ch = stream->readByte()))
			str += ch;
		debug("        %2u: %2u, \"%s\"", k, idx, str.c_str());
	}
	return count;
}

void loadBitmap4(Graphics::Surface& surf, uint16 tw, uint16 th, uint32 toffset, Common::SeekableReadStream* stream) {
	uint16 outPitch = tw>>1;
	surf.create(outPitch, th, Graphics::PixelFormat::createFormatCLUT8());
	byte *data = (byte *)surf.getPixels();
	
	stream->skip(toffset>>1);
	stream->read(data, uint32(outPitch)*th);
}

void loadBitmap8(Graphics::Surface& surf, uint16 tw, uint16 th, uint32 toffset, Common::SeekableReadStream* stream) {
	uint16 outPitch = tw;
	surf.create(outPitch, th, Graphics::PixelFormat::createFormatCLUT8());
	byte *data = (byte *)surf.getPixels();

	stream->skip(toffset);
	stream->read(data, uint32(outPitch)*th);
}

void convertBitmap(Graphics::Surface& surf, int size, Graphics::Surface& vga, Graphics::Surface& bin) {
	if (vga.h != 0) {
		byte *data, *vgap, *binp;
		data = (byte *)surf.getPixels();
		vgap = (byte *)vga.getPixels();
		binp = (byte *)bin.getPixels();
		for (int i=0; i<size; i+=2) {
			data[i+0]  = ((vgap[i>>1] & 0xF0)     );
			data[i+0] |= ((binp[i>>1] & 0xF0) >> 4);
			data[i+1]  = ((vgap[i>>1] & 0x0F) << 4);
			data[i+1] |= ((binp[i>>1] & 0x0F)     );
		}
	}
}
void convertBitmap(Graphics::Surface& surf, int size, Graphics::Surface& vga, Graphics::Surface& bin, Graphics::Surface& ma8) {
	if (ma8.h != 0) {
		byte *data, *ma8p;
		data = (byte *)surf.getPixels();
		ma8p = (byte *)ma8.getPixels();
		for (int i=0; i<size; i++)
			data[i] = ma8p[i];
	} else {
		convertBitmap(surf, size, vga, bin);
	}
}

PFont *_fntP;
FFont *_fntF;

uint16 loadTags(Common::SeekableReadStream& stream, char** &pstrs, uint16* &pidxs) {
	uint16 count;
	count = stream.readUint16LE();
	debug("        %u:", count);

	char **strs = new char *[count];
	assert(strs);
	uint16 *idxs = new uint16[count];
	assert(idxs);

	for (uint16 i=0; i<count; i++) {
		Common::String string;
		byte c = 0;
		uint16 idx;

		idx = stream.readUint16LE();
		while ((c = stream.readByte()))
			string += c;
		debug("        %2u: %2u, \"%s\"", i, idx, string.c_str());

		idxs[i] = idx;
		strs[i] = new char[string.size()+1];
		strcpy(strs[i], string.c_str());
	}
	pstrs = strs;
	pidxs = idxs;
	return count;
}

void parseFile(Common::Platform platform, Common::SeekableReadStream& file, const char* name, int resource) {
	const char *dot;
	DGDS_EX _ex;

	if ((dot = strrchr(name, '.'))) {
		_ex = MKTAG24(dot[1], dot[2], dot[3]);
	} else {
		_ex = 0;
	}

	uint parent = 0;

	DgdsParser ctx(file, name);
	if (isFlatfile(platform, _ex)) {
		uint16 tcount;
		uint16 *tw, *th;
		uint32 *toffset;
		Common::String line;
		
		switch (_ex) {
			case EX_RST: {
				uint32 mark;

				mark = file.readUint32LE();
				debug("    0x%X", mark);

				// elaborate guesswork. who knows it might be true.
				while (!file.eos()) {
					uint16 idx;
					uint16 vals[7];

					idx = file.readUint16LE();
					debugN("  #%u:\t", idx);
					if (idx == 0) break;
					for (int i=0; i<ARRAYSIZE(vals); i++) {
						vals[i] = file.readUint16LE();
						if (i != 0) debugN(", ");
						debugN("%u", vals[i]);
					}
					debug(".");
				}
				debug("-");

				while (!file.eos()) {
					uint16 idx;
					uint16 vals[2];
					idx = file.readUint16LE();
					debugN("  #%u:\t", idx);
					for (int i=0; i<ARRAYSIZE(vals); i++) {
						vals[i] = file.readUint16LE();
						if (i != 0) debugN(", ");
						debugN("%u", vals[i]);
					}
					debug(".");
					if (idx == 0) break;
				}
				debug("-");
				}
				break;
			case EX_SCR: {
				/* Unknown image format (Amiga). */
				byte tag[5];
				file.read(tag, 4);		/* maybe */
				tag[4] = '\0';

				uint16 pitch, planes;
				pitch = file.readUint16BE();	/* always 200 (320x200 screen). */
				planes = file.readUint16BE();	/* always 5 (32 color). */

				debug("    \"%s\" pitch:%u bpp:%u size: %u bytes",
						tag, pitch, planes,
						320*planes*200/8);

				if (resource == 0) {
				    //file.read(binData, file.size());
				}
				}
			        break;
			case EX_BMP: {
				/* Unknown image format (Amiga). */
				tcount = file.readUint16BE();
				tw = new uint16[tcount];
				th = new uint16[tcount];

				uint32 packedSize, unpackedSize;
				unpackedSize = file.readUint32BE();
				debug("        [%u] %u =", tcount, unpackedSize);

				uint32 sz = 0;
				toffset = new uint32[tcount];
				for (uint16 k=0; k<tcount; k++) {
					tw[k] = file.readUint16BE();
					th[k] = file.readUint16BE();
					debug("        %ux%u ~@%u", tw[k], th[k], sz);

					toffset[k] = sz;
					sz += uint(tw[k]+15)/16*th[k]*5;
				}
				debug("    ~= [%u]", sz);

				/* this is a wild guess. */
				byte version[13];
				file.read(version, 12);
				version[12] = '\0';
				debug("    %s", version);

				unpackedSize = file.readUint32BE();
				packedSize = file.readUint32BE();
				debug("        %u -> %u",
						packedSize, unpackedSize);

				if (resource >= 0) {
//				    file.read(_binData, file.size());

				    _tcount = tcount;
				    _tw = tw[resource];
				    _th = th[resource];
				    _toffset = toffset[resource];
				}
				}
				break;
			case EX_INS: {
				/* AIFF sound sample (Amiga). */
			        byte *dest = new byte[file.size()];
			        file.read(dest, file.size());
			        soundData = new Common::MemoryReadStream(dest, file.size(), DisposeAfterUse::YES);
				}
				break;
			case EX_SNG:
				/* IFF-SMUS music (Amiga). */
				break;
			case EX_AMG:
				/* (Amiga). */
				line = file.readLine();
				while (!file.eos() && !line.empty()) {
					debug("    \"%s\"", line.c_str());
					line = file.readLine();
				}
				break;
			case EX_VIN:
				line = file.readLine();
				while (!file.eos()) {
					if (!line.empty()) debug("    \"%s\"", line.c_str());
					line = file.readLine();
				}
				break;
			default:
				break;
		}
		int leftover = file.size()-file.pos();
		file.hexdump(leftover);
		file.skip(leftover);
	} else {
		uint16 tcount;
		uint16 scount;
		uint16 *tw = 0, *th = 0;
		uint32 *toffset = 0;
		
		uint16 *mtx;
		uint16 mw, mh;

		scount = 0;

		struct DgdsChunk chunk;
		while (chunk.readHeader(ctx)) {
			if (chunk.container) {
				parent = chunk._id;
				continue;
			}

			Common::SeekableReadStream *stream;
			
			bool packed = chunk.isPacked(_ex);
			stream = packed ? chunk.decodeStream(ctx) : chunk.readStream(ctx);

			/*
			 debug(">> %u:%u", stream->pos(), file->pos());*/

			if (resource == -1) {
				uint siz = stream->size();
				byte *dest = new byte[siz];
				Common::String cname = Common::String::format("%s:%s", name, chunk.id);

				Common::DumpFile out;
				if (!out.open(cname)) {
					debug("Couldn't write to %s", cname.c_str());
				} else {
					stream->read(dest, siz);
					out.write(dest, siz);
					stream->seek(0);
					out.close();
				}
			}

			switch (_ex) {
				case EX_TDS:
					/* Heart of China. */
					if (chunk.isSection(ID_THD)) {
						uint32 mark;

						mark = stream->readUint32LE();
						debug("    0x%X", mark);

						char version[7];
						stream->read(version, sizeof(version));
						debug("    \"%s\"", version);

						byte ch;
						Common::String bmpName;
						while ((ch = stream->readByte()))
							bmpName += ch;
						debug("    \"%s\"", bmpName.c_str());

						Common::String personName;
						while ((ch = stream->readByte()))
							personName += ch;
						debug("    \"%s\"", personName.c_str());
					}
					break;
				case EX_DDS:
					/* Heart of China. */
					if (chunk.isSection(ID_DDS)) {
						uint32 mark;

						mark = stream->readUint32LE();
						debug("    0x%X", mark);

						char version[7];
						stream->read(version, sizeof(version));
						debug("    \"%s\"", version);

						byte ch;
						Common::String tag;
						while ((ch = stream->readByte()))
							tag += ch;
						debug("    \"%s\"", tag.c_str());
					}
					break;
				case EX_SDS:
					if (chunk.isSection(ID_SDS)) {
						uint32 mark;

						mark = stream->readUint32LE();
						debug("    0x%X", mark);

						char version[7];
						stream->read(version, sizeof(version));
						debug("    %s", version);

						uint16 idx;
						idx = stream->readUint16LE();
						debug("    S%d.SDS", idx);


						_bubbles.clear();
						while (stream->pos() < stream->size()) {
							byte op;

							op = stream->readByte();
							if (op == 0x4) {
								uint16 op0, op1, op2;

								stream->seek(-1, SEEK_CUR);
								op0 = stream->readUint16LE();
								op1 = stream->readUint16LE();
								op2 = stream->readUint16LE();

								debug("0x%X 0x%X 0x%X", op0, op1, op2);
								if (op0 == 0x4 && op1 == 0x3 && op2 == 0x0) {
									uint16 unk0, unk1, unk2;
									uint16 count;

									unk0 = stream->readUint16LE();
									unk1 = stream->readUint16LE();
									unk2 = stream->readUint16LE();
									debug(" %d %d %d", unk0, unk1, unk2);

									count = stream->readUint16LE();
									Common::String str;
									for (uint16 k=0; k<count; k++) {
										byte ch;
										ch = stream->readByte();
										str += ch;
									}
									_bubbles.push_back(Common::String(str.c_str()));
									debug(" \"%s\"", str.c_str());
								} else {
									stream->seek(-5, SEEK_CUR);
								}
							} else {
								debug("0x%X '%c'", op, Common::isPrint(op) ? op : ' ');
							}
						}
					}
					break;
				case EX_TTM:
					if (chunk.isSection(ID_VER)) {
						char version[5];
						
						stream->read(version, sizeof(version));
						debug("        %s", version);
					} else if (chunk.isSection(ID_PAG)) {
						uint16 pages;
						pages = stream->readUint16LE();
						debug("        %u", pages);
					} else if (chunk.isSection(ID_TT3)) {
						if (resource == 0) {
							uint32 size = stream->size();
							byte *dest = new byte[size];
							stream->read(dest, size);
							ttm = new Common::MemoryReadStream(dest, size, DisposeAfterUse::YES);
							Common::strlcpy(ttmName, name, sizeof(ttmName));
						} else {
							while (!stream->eos()) {
								uint16 code;
								byte count;
								uint op;

								code = stream->readUint16LE();
								count = code & 0x000F;
								op = code & 0xFFF0;

								debugN("\tOP: 0x%4.4x %2u ", op, count);
								if (count == 0x0F) {
									Common::String sval;
									byte ch[2];

									do {
										ch[0] = stream->readByte();
										ch[1] = stream->readByte();
										sval += ch[0];
										sval += ch[1];
									} while (ch[0] != 0 && ch[1] != 0);

									debugN("\"%s\"", sval.c_str());
								} else {
									int ival;

									for (byte k=0; k<count; k++) {
										ival = stream->readSint16LE();

										if (k == 0)
											debugN("%d", ival);
										else
											debugN(", %d", ival);
									}
								}
								debug(" ");
							}
						}
					} else if (chunk.isSection(ID_TTI)) {
						debug("        %u instructions", chunk._size);
					} else if (chunk.isSection(ID_TAG)) {
						uint16 count;

						count = stream->readUint16LE();
						debug("        %u", count);
						// something fishy here. the first two entries sometimes are an empty string or non-text junk.
						// most of the time entries have text (sometimes with garbled characters).
						// this parser is likely not ok. but the NUL count seems to be ok.
						for (uint16 k=0; k<count; k++) {
							byte ch;
							uint16 idx;
							Common::String str;

							idx = stream->readUint16LE();
							while ((ch = stream->readByte()))
								str += ch;
							debug("        %2u: %2u, \"%s\"", k, idx, str.c_str());
						}
					}
					break;
				case EX_GDS:
					if (chunk.isSection(ID_INF)) {
						stream->hexdump(stream->size());
						uint32 mark;
						char version[7];

						mark = stream->readUint32LE();
						debug("    0x%X", mark);

						stream->read(version, sizeof(version));
						debug("    \"%s\"", version);
						
					} else if (chunk.isSection(ID_SDS)) {
						stream->hexdump(stream->size());

						uint32 x;
						x = stream->readUint32LE();
						debug("    %u", x);

						while (!stream->eos()) {
							uint16 x2;
							do {
								do {
									x2 = stream->readUint16LE();
									debugN("        %u: %u|%u, ", x2, (x2&0xF), (x2>>4));
									if (stream->pos() >= stream->size()) break;
								} while ((x2 & 0x80) != 0x80);
								debug("-");
								if (stream->pos() >= stream->size()) break;
							} while ((x2 & 0xF0) != 0xF0);
						}
					}
					break;
				case EX_ADS:
				case EX_ADL:
				case EX_ADH:
					if (chunk.isSection(ID_VER)) {
						char version[5];
						
						stream->read(version, sizeof(version));
						debug("        %s", version);
					} else if (chunk.isSection(ID_RES)) {
							debug("res0");
						if (resource == 0) {
							debug("res");
							_count = loadTags(*stream, _strs, _idxs);
						} else {
							readStrings(stream);
						}
					} else if (chunk.isSection(ID_SCR)) {
						if (resource == 0) {
							uint32 size = stream->size();
							byte *dest = new byte[size];
							stream->read(dest, size);
							ads = new Common::MemoryReadStream(dest, size, DisposeAfterUse::YES);
							Common::strlcpy(adsName, name, sizeof(adsName));
						} else {
							/* this is either a script, or a property sheet, i can't decide. */
							while (!stream->eos()) {
								uint16 code;
								code = stream->readUint16LE();
								if ((code&0xFF00) == 0) {
									uint16 tag = (code&0xFF);
									debug("          PUSH %d (0x%4.4X)", tag, tag); // ADS:TAG or TTM:TAG id.
								} else {
									const char *desc = "";
									switch (code) {
										case 0xF010:
										case 0xF200:
										case 0xFDA8:
										case 0xFE98:
										case 0xFF88:
										case 0xFF10:
											debug("          INT 0x%4.4X\t;", code);
											continue;

										case 0xFFFF:
											debug("          INT 0x%4.4X\t; return", code);
											debug("-");
											continue;

										case 0x0190:
										case 0x1070:
										case 0x1340:
										case 0x1360:
										case 0x1370:
										case 0x1420:
										case 0x1430:
										case 0x1500:
										case 0x1520:
										case 0x2000:
										case 0x2010:
										case 0x2020:
										case 0x3010:
										case 0x3020:
										case 0x30FF:
										case 0x4000:
										case 0x4010:	desc = "?";			break;

										case 0x1330:					break;
										case 0x1350:	desc = "? (res,rtag)";		break;

										case 0x1510:	desc = "? ()";			break;
										case 0x2005:	desc = "? (res,rtag,?,?)";	break;

										default:
											break;
									}
									debug("          OP 0x%4.4X\t;%s", code, desc);
								}
							}
							assert(stream->size()==stream->pos());
							stream->hexdump(stream->size());
						}
					} else if (chunk.isSection(ID_TAG)) {
						readStrings(stream);
					}
					break;
				case EX_REQ:
					if (parent == ID_TAG) {
						if (chunk.isSection(ID_REQ)) {
							readStrings(stream);
						} else if (chunk.isSection(ID_GAD)) {
							readStrings(stream);
						}
					} else if (parent == ID_REQ) {
						if (chunk.isSection(ID_REQ)) {
						} else if (chunk.isSection(ID_GAD)) {
						}
					}
					break;
				case EX_SNG:
					/* DOS. */
					if (chunk.isSection(ID_SNG)) {
						musicSize = stream->size();

						debug("        %2u: %u bytes", scount, musicSize);

						musicData = (uint8 *)malloc(musicSize);
						stream->read(musicData, musicSize);
						scount++;

						if (resource == -1) {
							Common::String cname = Common::String::format("%s:%u%s", name, scount, ".SND");
							Common::DumpFile out;
							if (!out.open(cname)) {
								debug("Couldn't write to %s", cname.c_str());
							} else {
								out.write(musicData, musicSize);
								out.close();
							}
						}
					} else if (chunk.isSection(ID_INF)) {
						uint32 count;
						count = stream->size()/2;
						debug("        [%u]", count);
						for (uint32 k = 0; k < count; k++) {
							uint16 idx;
							idx = stream->readUint16LE();
							debug("        %2u: %u", k, idx);
						}
					}
					break;
				case EX_SX:
					/* Macintosh. */
					if (chunk.isSection(ID_INF)) {
						uint16 type, count;

						type = stream->readUint16LE();
						count = stream->readUint16LE();

						debug("        %u [%u]:", type, count);
						for (uint16 k = 0; k < count; k++) {
							uint16 idx;
							idx = stream->readUint16LE();
							debug("        %2u: %u", k, idx);
						}
					} else if (chunk.isSection(ID_TAG)) {
						readStrings(stream);
					} else if (chunk.isSection(ID_FNM)) {
						readStrings(stream);
					} else if (chunk.isSection(ID_DAT)) {
						uint16 idx, type;
						byte compression;
						uint32 unpackSize;
						idx = stream->readUint16LE();
						type = stream->readUint16LE();
						compression = stream->readByte();
						unpackSize = stream->readUint32LE();
						debug("        #%2u: (0x%X?) %s %u", idx, type, compressionDescr[compression], unpackSize);

						musicSize = unpackSize;
						debug("        %2u: %u bytes", scount, musicSize);

						musicData = (uint8 *)malloc(musicSize);
						decompress(compression, musicData, musicSize, *stream, stream->size()-stream->pos());

						scount++;

						if (resource == -1) {
							Common::String cname = Common::String::format("%s:%u%s", name, scount, ".SND");
							Common::DumpFile out;
							if (!out.open(cname)) {
								debug("Couldn't write to %s", cname.c_str());
							} else {
								out.write(musicData, musicSize);
								out.close();
							}
						}

					}
					break;
				case EX_PAL:
					/* DOS & Macintosh. */
					if (resource == 0) {
						if (chunk.isSection(ID_VGA)) {
							stream->read(palette, 256*3);
							
							for (uint k=0; k<256*3; k+=3) {
								palette[k+0] <<= 2;
								palette[k+1] <<= 2;
								palette[k+2] <<= 2;
							}
						}
					} else {
						if (chunk.isSection(ID_VGA)) {
							stream->skip(256*3);
						}
					}
					break;
				case EX_FNT:
					if (resource == 0) {
						if (chunk.isSection(ID_FNT)) {
							byte magic = stream->readByte();
							stream->seek(-1, SEEK_CUR);
							debug("    magic: %u", magic);

							if (magic != 0xFF)
								_fntF = FFont::load_Font(*stream);
							else
								_fntP = PFont::load_PFont(*stream);
						}
					}
					break;
				case EX_SCR:
					/* currently does not handle the VQT: and OFF: chunks
					 for the compressed pics in the DOS port. */
					if (resource == 0) {
						if (chunk.isSection(ID_BIN)) {
							loadBitmap4(binData, 320, 200, 0, stream);
						} else if (chunk.isSection(ID_VGA)) {
							loadBitmap4(vgaData, 320, 200, 0, stream);
						} else if (chunk.isSection(ID_MA8)) {
							loadBitmap8(ma8Data, 320, 200, 0, stream);
						} else if (chunk.isSection(ID_VQT)) {
							stream->skip(stream->size());
						}
					} else {
						if (chunk.isSection(ID_BIN)) {
							stream->skip(stream->size());
						} else if (chunk.isSection(ID_VGA)) {
							stream->skip(stream->size());
						} else if (chunk.isSection(ID_MA8)) {
							stream->skip(stream->size());
						} else if (chunk.isSection(ID_VQT)) {
							stream->skip(stream->size());
						}
					}
					break;
				case EX_BMP:
					/* currently does not handle the VQT: and OFF: chunks
					 for the compressed pics in the DOS port. */
					if (chunk.isSection(ID_INF)) {
						tcount = stream->readUint16LE();
						debug("        [%u] =", tcount);
						
						tw = new uint16[tcount];
						th = new uint16[tcount];
						for (uint16 k=0; k<tcount; k++)
							tw[k] = stream->readUint16LE();
						for (uint16 k=0; k<tcount; k++)
							th[k] = stream->readUint16LE();
						
						uint32 sz = 0;
						toffset = new uint32[tcount];
						for (uint16 k=0; k<tcount; k++) {
							debug("        %ux%u @%u", tw[k], th[k], sz);
							toffset[k] = sz;
							sz += uint32(tw[k])*th[k];
						}
						debug("        BIN|VGA: %u bytes", (sz+1)/2);
					} else if (chunk.isSection(ID_MTX)) {
						uint32 mcount;
						
						mw = stream->readUint16LE();
						mh = stream->readUint16LE();
						mcount = uint32(mw)*mh;
						debug("        %ux%u: %u bytes", mw, mh, mcount*2);
						
						mtx = new uint16[mcount];
						for (uint32 k=0; k<mcount; k++) {
							uint16 tile;
							tile = stream->readUint16LE();
							mtx[k] = tile;
							//								debug("        %u", tile);
						}
					}
					
					// DCORNERS.BMP, DICONS.BMP, HELICOP2.BMP, WALKAWAY.BMP, KARWALK.BMP, BLGREND.BMP, FLAMDEAD.BMP, W.BMP, ARCADE.BMP
					// MTX: SCROLL.BMP (intro title), SCROLL2.BMP
					if (resource >= 0) {
						if (chunk.isSection(ID_BIN)) {
							loadBitmap4(_binData, tw[resource], th[resource], toffset[resource], stream);
						} else if (chunk.isSection(ID_VGA)) {
							loadBitmap4(_vgaData, tw[resource], th[resource], toffset[resource], stream);
						} else if (chunk.isSection(ID_INF)) {
							_tcount = tcount;
							_tw = tw[resource];
							_th = th[resource];
							_toffset = toffset[resource];
						} else if (chunk.isSection(ID_MTX)) {
							_mtx = mtx;
							_mw = mw;
							_mh = mh;
						}
					}
					if (chunk.isSection(ID_BIN)) {
						stream->skip(stream->size()-stream->pos());
					} else if (chunk.isSection(ID_VGA)) {
						stream->skip(stream->size()-stream->pos());
					} else if (chunk.isSection(ID_VQT)) {
						stream->skip(stream->size()-stream->pos());
					}
					break;
				default:
					break;
			}
			int leftover = stream->size()-stream->pos();
			stream->hexdump(leftover);
			stream->skip(leftover);
			
			delete stream;
		}
	}

	if (_ex == EX_BMP) {
		BMPs.push_back(Common::String(name));
		debug("BMPs: %s", name);
	}
#if 0
	if (_ex == EX_PAL) {
		Common::DumpFile out;

		Common::String cname = Common::String::format("%s.gpl", name);

		if (!out.open(cname)) {
			debug("Couldn't write to %s", cname.c_str());
		} else {
			Common::String header = Common::String::format("GIMP Palette\nName: %s\nColumns: 0\n#\n", name);
			out.writeString(header);
			for (uint i=0; i<256*3; i+=3) {
				Common::String color = Common::String::format("%3u %3u %3u\tUntitled\n",
						palette[i+0], palette[i+1], palette[i+2]);
				out.writeString(color);
			}
			out.close();
		}
		
	} else if (_ex == EX_SCR) {
		byte *vgaData_;
		byte *binData_;
		byte *ma8Data_;

		byte *scrData_;
		scrData_ = (byte *)bottomBuffer.getPixels();

		if (ma8Data.h != 0) {
			ma8Data_ = (byte *)ma8Data.getPixels();
			for (int i=0; i<sw*sh; i++) {
				scrData_[i] = ma8Data_[i];
			}
		} else if (vgaData.h != 0) {
			vgaData_ = (byte *)vgaData.getPixels();
			binData_ = (byte *)binData.getPixels();
			for (int i=0; i<sw*sh; i+=2) {
				scrData_[i+0]  = ((vgaData_[i>>1] & 0xF0)     );
				scrData_[i+0] |= ((binData_[i>>1] & 0xF0) >> 4);
				scrData_[i+1]  = ((vgaData_[i>>1] & 0x0F) << 4);
				scrData_[i+1] |= ((binData_[i>>1] & 0x0F)     );
			}
		}

		Common::DumpFile out;

		uint siz = sw*sh;
		Common::String cname = Common::String::format("%s.data", name);

		if (!out.open(cname)) {
			debug("Couldn't write to %s", cname.c_str());
		} else {
			out.write(scrData_, siz);
			out.close();
		}
	}
#endif
	debug("  [%u:%u] --", file.pos(), ctx.bytesRead);
}

static void explode(Common::Platform platform, const char *indexName, const char *fileName, int resource) {
	if (fileName) {
		Common::File file;
		if (file.open(fileName)) {
			parseFile(platform, file, fileName, resource);
			file.close();
			return;
		}	
	}

	Common::File index, volume;
	Common::SeekableSubReadStream *file;
	if (index.open(indexName)) {
		byte salt[4];
		uint16 nvolumes;

		index.read(salt, sizeof(salt));
		nvolumes = index.readUint16LE();

		if (!fileName)
			debug("(%u,%u,%u,%u) %u", salt[0], salt[1], salt[2], salt[3], nvolumes);

		for (uint i=0; i<nvolumes; i++) {
			char name[DGDS_FILENAME_MAX+1];
			uint16 nfiles;

			index.read(name, sizeof(name));
			name[DGDS_FILENAME_MAX] = '\0';
			
			nfiles = index.readUint16LE();
			
			debugN("--\n#%u %s, %u files", i, name, nfiles);

			if (!volume.open(name)) {
				debug(", failed to open");
				continue;
			}

			debug(", %d bytes", volume.size());

			for (uint j=0; j<nfiles; j++) {
				int32 hash;
				uint32 offset, fileSize;

				hash = index.readSint32LE();
				offset = index.readUint32LE();

				volume.seek(offset);
				volume.read(name, sizeof(name));
				name[DGDS_FILENAME_MAX] = '\0';
				fileSize = volume.readUint32LE();

				if (!fileName || scumm_stricmp(name, fileName) == 0)
					debug("  #%u %s 0x%X=0x%X %u %u\n  --", j, name, hash, dgdsHash(name, salt), offset, fileSize);

				if (fileSize == 0xFFFFFFFF) {
					continue;
				}

				if (fileName && scumm_stricmp(name, fileName)) {
					volume.skip(fileSize);
					continue;
				}

				file = new Common::SeekableSubReadStream(&volume, volume.pos(), volume.pos()+fileSize, DisposeAfterUse::NO);

				if (resource == -1) {
					char *buf = new char[fileSize];
					Common::DumpFile out;
					if (!out.open(name)) {
						debug("Couldn't write to %s", name);
					} else {
						file->read(buf, fileSize);
						out.write(buf, fileSize);
						out.close();
						file->seek(0);
					}
					delete [] buf;
				}
				
				parseFile(platform, *file, name, resource);

				if (!fileName)
					debug("  #%u %s %d .", j, name, volume.pos());

				if (fileName) {
				    volume.close();
				    index.close();
				    return;
				}
			}
			volume.close();
		}
		index.close();
	}	
}

int bw = 0, bh = 0;
int bk = -1;

char bmpNames[16][DGDS_FILENAME_MAX+1];
char scrNames[16][DGDS_FILENAME_MAX+1];

int id = 0, sid = 0;
const Common::Rect rect(0, 0, sw, sh);

int delay = 0;
Common::Rect drawWin(0, 0, sw, sh);

Common::String text;

// TAGS vs UNIQUE TAGS
// HASHTABLE?

uint32 lookupVolume(const char *rmfName, const char* filename, char* volname) {
	Common::File index;
	if (!index.open(rmfName)) return 0xFFFFFFFF;

	byte salt[4];
	int32 filehash;

	index.read(salt, sizeof(salt));
	filehash = dgdsHash(filename, salt);

	uint16 nvolumes;
	nvolumes = index.readUint16LE();
	for (uint i=0; i<nvolumes; i++) {
		index.read(volname, DGDS_FILENAME_MAX+1);
		volname[DGDS_FILENAME_MAX] = '\0';

		uint16 nfiles;
		nfiles = index.readUint16LE();
		for (uint j=0; j<nfiles; j++) {
			int32 hash = index.readSint32LE();
			uint32 offset = index.readUint32LE();
			if (hash == filehash) {
				index.close();
				return offset;
			}
		}
	}
	index.close();
	return 0xFFFFFFFF;
}

static
Common::SeekableReadStream *createReadStream(const char *rmfName, const char *filename) {
	char volname[DGDS_FILENAME_MAX+1];
	uint32 offset;
	offset = lookupVolume(rmfName, filename, volname);
	if (offset == 0xFFFFFFFF)
		return 0;

	Common::File *volume = new Common::File;
	do {
		if (!volume->open(volname))
			break;
		volume->seek(offset);
		volume->read(volname, sizeof(volname));
		volname[DGDS_FILENAME_MAX] = '\0';

		uint32 fileSize;
		fileSize = volume->readUint32LE();

		if (fileSize == 0xFFFFFFFF)
			break;
		if (scumm_stricmp(volname, filename))
			break;
		return new Common::SeekableSubReadStream(volume, volume->pos(), volume->pos()+fileSize, DisposeAfterUse::YES);
	} while (0);

	delete volume;
	return 0;
}

struct TTMData {
	char filename[13];

	Common::SeekableReadStream *scr;
	Common::String ver;

	void init(const char *filename_) {
		Common::strlcpy(filename, filename_, sizeof(filename));
		scr = 0;
		ver.clear();
	}
};

struct TTMState {
	const TTMData *dataPtr;
	uint16 scene;
	int delay;
};

class TTMInterpreter {
public:
	TTMInterpreter(DgdsEngine *vm);

	bool load(const char *filename, TTMData *data);
	void unload(TTMData *data);

	bool callback(DgdsChunk &chunk);
	
	void init(TTMState *scriptState, const TTMData *scriptData);
	bool run(TTMState *script);

protected:
	DgdsEngine *_vm;

	const char *_filename;
	TTMData *_scriptData;
};

TTMInterpreter::TTMInterpreter(DgdsEngine* vm) : _vm(vm), _scriptData(0), _filename(0) {}

bool TTMInterpreter::load(const char *filename, TTMData *scriptData) {
	Common::SeekableReadStream *stream = createReadStream(_vm->_rmfName, filename);

	if (!stream) {
		error("Couldn't open script file '%s'", filename);
		return false;
	}

	_scriptData = scriptData;
	_filename = filename;
	_scriptData->init(_filename);

        Common::Functor1Mem< DgdsChunk &, bool, TTMInterpreter > c(this, &TTMInterpreter::callback);
	DgdsParser dgds(*stream, _filename);
	dgds.parse(c);

	delete stream;

	_scriptData = 0;
	_filename = 0;
	return true;
}

void TTMInterpreter::unload(TTMData *data) {
	if (!data)
		return;
	delete data->scr;

	data->init("");
}

void TTMInterpreter::init(TTMState *state, const TTMData *data) {
	state->dataPtr = data;
	state->delay = 0;	
	state->scene = 0;
	data->scr->seek(0);
}

bool TTMInterpreter::run(TTMState *script) {
	if (!script) return false;

	Common::SeekableReadStream *scr = script->dataPtr->scr;
	if (!scr) return false;
	if (scr->pos() >= scr->size()) return false;

	script->delay = 0;

	int ki = 0;
	do {
		uint16 code;
		byte count;
		uint op;
		int16 ivals[8];

		Common::String sval;

		code = scr->readUint16LE();
		count = code & 0x000F;
		op = code & 0xFFF0;

		debugN("\tOP #%4d: 0x%4.4x %2u ", ki, op, count);
		Common::String txt;
		txt += Common::String::format("OP: 0x%4.4x %2u ", op, count);
		if (count == 0x0F) {
			byte ch[2];

			do {
				ch[0] = scr->readByte();
				ch[1] = scr->readByte();
				sval += ch[0];
				sval += ch[1];
			} while (ch[0] != 0 && ch[1] != 0);

			debugN("\"%s\"", sval.c_str());
			txt += "\"" + sval + "\"";
		} else {
			int16 ival;

			for (byte i=0; i<count; i++) {
				ival = scr->readSint16LE();
				ivals[i] = ival;

				if (i == 0) {
					debugN("%d", ival);
					txt += Common::String::format("%d", ival);
				} else {
					debugN(", %d", ival);
					txt += Common::String::format(", %d", ival);
				}
			}
		}
		debug(" ");
		ki++;

		Common::Rect bmpWin(0, 0, sw, sh);

		switch (op) {
			case 0x0000:
				// FINISH:	void
				break;

			case 0xf010:
				// LOAD SCR:	filename:str
				Common::strlcpy(scrNames[sid], sval.c_str(), sizeof(scrNames[sid]));
				vgaData.free();
				binData.free();
				ma8Data.free();
				explode(_vm->_platform, _vm->_rmfName, scrNames[sid], 0);
				convertBitmap(bottomBuffer, sw*sh, vgaData, binData, ma8Data);
//				resData.copyRectToSurface(scrData, 0, 0, rect);
				continue;
			case 0xf020:
				// LOAD BMP:	filename:str
				Common::strlcpy(bmpNames[id], sval.c_str(), sizeof(bmpNames[id]));
				continue;
			case 0xf050:
				// LOAD PAL:	filename:str
				explode(_vm->_platform, _vm->_rmfName, sval.c_str(), 0);
				continue;
			case 0xf060:
				// LOAD SONG:	filename:str
				if (_vm->_platform == Common::kPlatformAmiga) {
					const char *fileName = "DYNAMIX.INS";
					byte volume = 255;
					byte channel = 0;
					_vm->stopSfx(channel);
					_vm->playSfx(fileName, channel, volume);
				} else {
					_vm->playMusic(sval.c_str());
				}
				continue;

			case 0x1030:
				// SET BMP:	id:int [-1:n]
				bk = ivals[0];

				_vgaData.free();
				_binData.free();
				if (bk != -1) {
					explode(_vm->_platform, _vm->_rmfName, bmpNames[id], bk);
					bw = _tw; bh = _th;
					convertBitmap(_bmpData, bw*bh, _vgaData, _binData);
				}
				continue;
			case 0x1050:
				// SELECT BMP:	    id:int [0:n]
				id = ivals[0];
				continue;
			case 0x1060:
				// SELECT SCR|PAL:  id:int [0]
				sid = ivals[0];
				continue;
			case 0x1090:
				// SELECT SONG:	    id:int [0]
				continue;

			case 0x4120:
				// FADE IN:	?,?,?,?:byte
				g_system->getPaletteManager()->setPalette(palette, 0, 256);
				continue;

			case 0x4110:
				// FADE OUT:	?,?,?,?:byte
				g_system->delayMillis(script->delay);
				g_system->getPaletteManager()->setPalette(blacks, 0, 256);
				bottomBuffer.fillRect(rect, 0);
				continue;

			// these 3 ops do interaction between the topBuffer (imgData) and the bottomBuffer (scrData) but... it might turn out this uses z values!
			case 0xa050: {//GFX?	    i,j,k,l:int	[i<k,j<l] // HAPPENS IN INTRO.TTM:INTRO9
				// it works like a bitblit, but it doesn't write if there's something already at the destination?
				resData.blitFrom(bottomBuffer);
				resData.transBlitFrom(topBuffer);
				topBuffer.copyFrom(resData);
				continue;
				}
			case 0x0020: {//SAVE BG?:    void // OR PERHAPS SWAPBUFFERS ; it makes bmpData persist in the next frames.
				bottomBuffer.copyFrom(topBuffer);
				}
				continue;
			case 0x4200: {
				// STORE AREA:	x,y,w,h:int [0..n]		; it makes this area of bmpData persist in the next frames.
				const Common::Rect destRect(ivals[0], ivals[1], ivals[0]+ivals[2], ivals[1]+ivals[3]);
				resData.blitFrom(bottomBuffer);
				resData.transBlitFrom(topBuffer);
				bottomBuffer.copyRectToSurface(resData, destRect.left, destRect.top, destRect);
				}
				continue;

			case 0x0ff0: {
				// REFRESH:	void
				resData.blitFrom(bottomBuffer);
				Graphics::Surface bmpSub = topBuffer.getSubArea(bmpWin);
				resData.transBlitFrom(bmpSub, Common::Point(bmpWin.left, bmpWin.top));
				topBuffer.fillRect(bmpWin, 0);

				if (!text.empty()) {
					Common::StringArray lines;
					const int h = _fntP->getFontHeight();

					_fntP->wordWrapText(text, 200, lines);
					Common::Rect r(0, 7, sw, h*lines.size()+13);
					resData.fillRect(r, 15);
					for (uint i=0; i<lines.size(); i++) {
						const int w = _fntP->getStringWidth(lines[i]);
						_fntP->drawString(&resData, lines[i], 10, 10+1+i*h, w, 0);
					}
				}
				debug("FLUSH");
				}
				break;

			case 0xa520:
				//DRAW BMP: x,y:int ; happens once in INTRO.TTM
			case 0xa500: {
				debug("DRAW \"%s\"", bmpNames[id]);

				// DRAW BMP: x,y,tile-id,bmp-id:int [-n,+n] (CHINA)
				// This is kind of file system intensive, will likely have to change to store all the BMPs.
				if (count == 4) {
					_vgaData.free();
					_binData.free();
					bk = ivals[2];
					id = ivals[3];
					if (bk != -1) {
						explode(_vm->_platform, _vm->_rmfName, bmpNames[id], bk);
						bw = _tw; bh = _th;
						convertBitmap(_bmpData, bw*bh, _vgaData, _binData);
					} else {
					    bw = bh = 0;
					}
				}

				// DRAW BMP: x,y:int [-n,+n] (RISE)
				const Common::Rect destRect(ivals[0], ivals[1], ivals[0]+bw, ivals[1]+bh);
				Common::Rect clippedDestRect(0, 0, sw, sh);
				clippedDestRect.clip(destRect);
				clippedDestRect.clip(drawWin);

				if (bk != -1) {
					// DRAW BMP: x,y:int [-n,+n] (RISE)
					const Common::Point croppedBy(clippedDestRect.left-destRect.left, clippedDestRect.top-destRect.top);

					const int rows = clippedDestRect.height();
					const int columns = clippedDestRect.width();

					byte *src = (byte *)_bmpData.getPixels() + croppedBy.y * bw + croppedBy.x;
					byte *ptr = (byte *)topBuffer.getBasePtr(clippedDestRect.left, clippedDestRect.top);
					for (int i=0; i<rows; ++i) {
						for (int j=0; j<columns; ++j) {
							if (src[j])
								ptr[j] = src[j];
						}
						ptr += topBuffer.pitch;
						src += bw;
					}
				} else {
//					bmpData.fillRect(rect, 0);?
				}
				}
				continue;

			case 0x1110: {//SET SCENE?:  i:int   [1..n]
				// DESCRIPTION IN TTM TAGS.
				debug("SET SCENE: %u", ivals[0]);
				script->scene = ivals[0];

				if (!_bubbles.empty()) {
					if (!scumm_stricmp(script->dataPtr->filename, "INTRO.TTM")) {
						switch (ivals[0]) {
							case 15:	text = _bubbles[3];	break;
							case 16:	text = _bubbles[4];	break;
							case 17:	text = _bubbles[5];	break;
							case 19:	text = _bubbles[6];	break;
							case 20:	text = _bubbles[7];	break;
							case 22:	text = _bubbles[8];	break;
							case 23:	text = _bubbles[9];	break;
							case 25:	text = _bubbles[10];	break;
							case 26:	text = _bubbles[11];	break;
							default:	text.clear();		break;
						}
					} else if (!scumm_stricmp(script->dataPtr->filename, "BIGTV.TTM")) {
						switch (ivals[0]) {
							case 1:		text = _bubbles[0];	break;
							case 2:		text = _bubbles[1];	break;
							case 3:		text = _bubbles[2];	break;
						}
					}
					if (!text.empty()) script->delay += 1500;
				} else {
					text.clear();
				}
				/*
				scrData.fillRect(rect, 0);
				bmpData.fillRect(rect, 0);*/
				}
				continue;

			case 0x4000:
				//SET WINDOW? x,y,w,h:int	[0..320,0..200]
				drawWin = Common::Rect(ivals[0], ivals[1], ivals[2], ivals[3]);
				continue;

			case 0xa100:
				//SET WINDOW? x,y,w,h:int	[0..320,0..200]
				bmpWin = Common::Rect(ivals[0], ivals[1], ivals[0]+ivals[2], ivals[1]+ivals[3]);
				continue;

			case 0x1020: //DELAY?:	    i:int   [0..n]
				script->delay += ivals[0]*10;
				continue;

			case 0x10a0:
				// SET SCR|PAL:	    id:int [0]
			case 0x2000: //SET FRAME1?: i,j:int [0..255]

			case 0xa530:	// CHINA
				// DRAW BMP4:	x,y,tile-id,bmp-id:int	[-n,+n] (CHINA)
				// arguments similar to DRAW BMP but it draws the same BMP multiple times with radial simmetry? you can see this in the Dynamix logo star.

			case 0x0110: //PURGE BMPs?  void
			case 0x0080: //DRAW BG:	    void
			case 0x1100: //?	    i:int   [9]
			case 0x1300: //?	    i:int   [72,98,99,100,107] XPTO
			case 0x1310: //?	    i:int   [107]

			default:
				warning("Unimplemented TTM opcode: 0x%04X", op);
				continue;
		}
		break;
	} while (scr->pos() < scr->size());

//	g_system->displayMessageOnOSD(txt.c_str());
#if 0
	const Graphics::Font *font = FontMan.getFontByUsage(Graphics::FontManager::kConsoleFont);
	int w = font->getStringWidth(txt);
	/*
	int x = sh - w - 5; // lx + logo->w - w + 5;
	int y = sh - font->getFontHeight() - 5; //ly + logo->h + 5;*/

	dst = g_system->lockScreen();
	Common::Rect r(0, 7, sw, font->getFontHeight()+13);
	dst->fillRect(r, 0);
	font->drawString(dst, txt, 10, 10, w, 2);
	g_system->unlockScreen();
#endif
	Graphics::Surface *dst;
	dst = g_system->lockScreen();
	dst->copyRectToSurface(resData, 0, 0, rect);
	g_system->unlockScreen();
	g_system->updateScreen();
	g_system->delayMillis(script->delay);
	return true;
}

bool TTMInterpreter::callback(DgdsChunk &chunk) {
	Common::SeekableReadStream *stream = chunk._stream;
	switch (chunk._id) {
	case MKTAG24('P','A','G'): {
		uint16 pages;
		pages = stream->readUint16LE();
		debug("%u", pages);
	        }
		break;
	case MKTAG24('V','E','R'): {
		char version[5];
		stream->read(version, sizeof(version));
		debug("%s", version);
	        }
		break;
	case MKTAG24('T','A','G'): {
		readStrings(stream);
	        }
		break;
	case MKTAG24('T','T','3'):
		_scriptData->scr = stream->readStream(stream->size());
		break;
	case MKTAG24('T','T','I'):
		debug("%u instructions", chunk._size);
		break;
	default:
		warning("Unexpected chunk '%s' of size %d found in file '%s'", tag2str(chunk._id), chunk._size, _filename);
		break;
	}
	return false;
}

struct ADSData {
	char filename[13];

	uint16 count;
	char **names;
	TTMData *scriptDatas;

	Common::SeekableReadStream *scr;
	Common::String ver;

	void init(const char *filename_) {
		Common::strlcpy(filename, filename_, sizeof(filename));
		count = 0;
		names = 0;
		scriptDatas = 0;
		scr = 0;
		ver.clear();
	}
};

struct ADSState {
	const ADSData *dataPtr;
	uint16 scene;
	uint16 subIdx, subMax;

	TTMState *scriptStates;
};

class ADSInterpreter {
public:
	ADSInterpreter(DgdsEngine *vm);

	bool load(const char *filename, ADSData *data);
	void unload(ADSData *data);

	bool callback(DgdsChunk &chunk);
	
	void init(ADSState *scriptState, const ADSData *scriptData);
	bool run(ADSState *script);

protected:
	DgdsEngine *_vm;

	const char *_filename;
	ADSData *_scriptData;
};

ADSInterpreter::ADSInterpreter(DgdsEngine* vm) : _vm(vm), _scriptData(0), _filename(0) {}

bool ADSInterpreter::load(const char *filename, ADSData *scriptData) {
	Common::SeekableReadStream *stream = createReadStream(_vm->_rmfName, filename);

	if (!stream) {
		error("Couldn't open script file '%s'", filename);
		return false;
	}

	_scriptData = scriptData;
	_filename = filename;
	_scriptData->init(_filename);

        Common::Functor1Mem< DgdsChunk &, bool, ADSInterpreter > c(this, &ADSInterpreter::callback);
	DgdsParser dgds(*stream, _filename);
	dgds.parse(c);

	delete stream;

	TTMInterpreter interp(_vm);

	TTMData *scriptDatas;
	scriptDatas = new TTMData[_scriptData->count];
	assert(scriptDatas);
	_scriptData->scriptDatas = scriptDatas;

	for (uint16 i = _scriptData->count; i--; )
		interp.load(_scriptData->names[i], &_scriptData->scriptDatas[i]);

	_scriptData = 0;
	_filename = 0;
	return true;
}

void ADSInterpreter::unload(ADSData *data) {
	if (!data)
		return;
	for (uint16 i = data->count; i--; )
		delete data->names[i];
	delete data->names;
	delete data->scriptDatas;
	delete data->scr;

	data->init("");
}

void ADSInterpreter::init(ADSState *state, const ADSData *data) {
	state->dataPtr = data;
	state->scene = 0;
	state->subIdx = 0;
	state->subMax = 0;
	data->scr->seek(0);

	TTMInterpreter interp(_vm);

	TTMState *scriptStates;
	scriptStates = new TTMState[data->count];
	assert(scriptStates);
	state->scriptStates = scriptStates;

	for (uint16 i = data->count; i--; )
		interp.init(&state->scriptStates[i], &data->scriptDatas[i]);
}

bool ADSInterpreter::run(ADSState *script) {
	TTMInterpreter interp(_vm);

	if (script->subMax != 0) {
		TTMState *state = &script->scriptStates[script->subIdx-1];
		if (!interp.run(state) || state->scene >= script->subMax)
			script->subMax = 0;
		return true;
	}

	if (!script) return false;
	Common::SeekableReadStream *scr = script->dataPtr->scr;
	if (!scr) return false;
	if (scr->pos() >= scr->size()) return false;

	do {
		uint16 code;
		code = scr->readUint16LE();

		if ((code&0xFF00) == 0) {
			continue;
		}

		switch (code) {
		case 0x2005:
			// ADD_TTM
			{
				uint16 args[4];
				for (uint i=0; i<ARRAYSIZE(args); i++) {
					args[i] = scr->readUint16LE();
				}
				script->subIdx = args[0];
				script->subMax = args[1];
				debug("\tAI: 0x%4.4x %d %d %d %d", code, args[0], args[1], args[2], args[3]);
			}
			return true;
		case 0xF010:
		case 0xF200:
		case 0xFDA8:
		case 0xFE98:
		case 0xFF88:
		case 0xFF10:
		case 0xFFFF:
		case 0x0190:
		case 0x1070:
		case 0x1340:
		case 0x1360:
		case 0x1370:
		case 0x1420:
		case 0x1430:
		case 0x1500:
		case 0x1520:
		case 0x2000:
		case 0x2010:
		case 0x2020:
		case 0x3010:
		case 0x3020:
		case 0x30FF:
		case 0x4000:
		case 0x4010:
		case 0x1510:
		case 0x1330:
		case 0x1350:
		default:
			warning("Unimplemented ADS opcode: 0x%04X", code);
			continue;
		}
		break;
	} while (scr->pos() < scr->size());
	return false;
}

bool ADSInterpreter::callback(DgdsChunk &chunk) {
	Common::SeekableReadStream *stream = chunk._stream;
	switch (chunk._id) {
	case MKTAG24('A','D','S'):
		break;
	case MKTAG24('R','E','S'): {
	   	/* numbered list of used TTM resources. */
		uint16 count = stream->readUint16LE();
		char **strs = new char *[count];
		assert(strs);

		_scriptData->count = count;
		for (uint16 i=0; i<count; i++) {
			Common::String string;
			byte c = 0;
			uint16 idx;
			
			idx = stream->readUint16LE();
			assert(idx == (i+1));

			while ((c = stream->readByte()))
				string += c;

			strs[i] = new char[string.size()+1];
			strcpy(strs[i], string.c_str());
		}
		_scriptData->names = strs;
		}
		break;
	case MKTAG24('S','C','R'):
		_scriptData->scr = stream->readStream(stream->size());
		break;
	case MKTAG24('T','A','G'): {
		readStrings(stream);
	        }
		break;
	case MKTAG24('V','E','R'): {
		char version[5];
		stream->read(version, sizeof(version));
		debug("%s", version);
	        }
		break;
	default:
		warning("Unexpected chunk '%s' of size %d found in file '%s'", tag2str(chunk._id), chunk._size, _filename);
		break;
	}
	return false;
}

struct Channel {
	Audio::AudioStream *stream;
	Audio::SoundHandle handle;
	byte volume;
};

struct Channel _channels[2];

void DgdsEngine::playSfx(const char* fileName, byte channel, byte volume) {
	explode(_platform, _rmfName, fileName, 0);
	if (soundData) {
		Channel *ch = &_channels[channel];
		Audio::AudioStream *input = Audio::makeAIFFStream(soundData, DisposeAfterUse::YES);
		_mixer->playStream(Audio::Mixer::kSFXSoundType, &ch->handle, input, -1, volume);
		soundData = 0;
	}
}

void DgdsEngine::stopSfx(byte channel) {
	if (_mixer->isSoundHandleActive(_channels[channel].handle)) {
	    _mixer->stopHandle(_channels[channel].handle);
		_channels[channel].stream = 0;
	}
}

bool DgdsEngine::playPCM(byte *data, uint32 size) {
	_mixer->stopAll();

	if (!data) return false;

	byte numParts;
	byte *trackPtr[0xFF];
	uint16 trackSiz[0xFF];
	numParts = loadSndTrack(DIGITAL_PCM, trackPtr, trackSiz, data, size);
	if (numParts == 0) return false;

	for (byte part = 0; part < numParts; part++) {
		byte *ptr = trackPtr[part];

		bool digital_pcm = false;
		if (READ_LE_UINT16(ptr) == 0x00FE) {
			digital_pcm = true;
		}
		ptr += 2;

		if (!digital_pcm) continue;

		uint16 rate, length, first, last;
		rate = READ_LE_UINT16(ptr);

		length = READ_LE_UINT16(ptr+2);
		first = READ_LE_UINT16(ptr+4);
		last = READ_LE_UINT16(ptr+6);
		ptr += 8;

		ptr += first;
		debug(" - Digital PCM: %u Hz, [%u]=%u:%u",
				rate, length, first, last);
		trackPtr[part] = ptr;
		trackSiz[part] = length;

		Channel *ch = &_channels[part];
		byte volume = 255;
		Audio::AudioStream *input = Audio::makeRawStream(trackPtr[part], trackSiz[part],
				rate, Audio::FLAG_UNSIGNED, DisposeAfterUse::NO);
		_mixer->playStream(Audio::Mixer::kSFXSoundType, &ch->handle, input, -1, volume);
	}
	return true;
}

void DgdsEngine::playMusic(const char* fileName) {
	//stopMusic();

	explode(_platform, _rmfName, fileName, 0);
	if (musicData) {
		uint32 tracks;
		tracks = availableSndTracks(musicData, musicSize);
		if ((tracks & TRACK_MT32))
			_midiPlayer->play(musicData, musicSize);
		if ((tracks & DIGITAL_PCM))
			playPCM(musicData, musicSize);
	}
}
#if 0
int prev_id = -1;
int prev_bk = -1;
int prev_palette = -1;

void browseInit(Common::Platform _platform, const char *_rmfName, DgdsEngine* syst) {
	BMPs.clear();
	explode(_platform, _rmfName, 0, -1);
	bk = 0;
	id = 0;
	sid = 1;
}

void browse(Common::Platform _platform, const char *_rmfName, DgdsEngine* syst) {
	if (prev_id != id || prev_bk != bk) {
		explode(_platform, _rmfName, BMPs[id].c_str(), bk);
		prev_id = id;
		prev_bk = bk;

		Common::String txt = Common::String::format("%s: %ux%u (%u/%u)", BMPs[id].c_str(), _tw, _th, bk+1, _tcount);
		g_system->displayMessageOnOSD(txt.c_str());
	}
	if (prev_palette != sid) {
		sid = sid&3;
		prev_palette = sid;

		switch (sid) {
		case 3:
			for (uint i=0; i<256; i++) {
				palette[i*3+0] = i;
				palette[i*3+1] = i;
				palette[i*3+2] = i;
			}
			break;
		case 2:
			for (uint i=0; i<256; i++) {
				palette[i*3+0] = 255-i;
				palette[i*3+1] = 255-i;
				palette[i*3+2] = 255-i;
			}
			break;
		case 1:
			explode(_platform, _rmfName, "DRAGON.PAL", 0);
			break;
		}
		g_system->getPaletteManager()->setPalette(palette, 0, 256);
	}

	if (bk != -1) {
		if (_vgaData.h != 0) {
			byte *_bmpData_;
			_bmpData_ = (byte *)_bmpData.getPixels();
			byte *vgaData_;
			byte *binData_;
			bw = _tw; bh = _th;
			vgaData_ = (byte *)_vgaData.getPixels();
			binData_ = (byte *)_binData.getPixels();

			for (int i=0; i<bw*bh; i+=2) {
				_bmpData_[i+0]  = ((vgaData_[i>>1] & 0xF0)     );
				_bmpData_[i+0] |= ((binData_[i>>1] & 0xF0) >> 4);
				_bmpData_[i+1]  = ((vgaData_[i>>1] & 0x0F) << 4);
				_bmpData_[i+1] |= ((binData_[i>>1] & 0x0F)     );
			}

			const int rows = sh;
			const int columns = sw;

			byte *src = _bmpData_;
			byte *ptr = (byte *)bottomBuffer.getBasePtr(0, 0);
			for (int i=0; i<rows; ++i) {
				for (int j=0; j<columns; ++j) {
					ptr[j] = src[j];
				}
				ptr += bottomBuffer.pitch;
				src += bw;
			}
		}

		resData.clear(0);
		Common::Rect bmpWin(0, 0, _tw, _th);
		Graphics::Surface bmpSub = bottomBuffer.getSubArea(bmpWin);
		resData.blitFrom(bmpSub, Common::Point(bmpWin.left, bmpWin.top));
		resData.frameRect(bmpWin, 1);

		Graphics::Surface *dst;
		dst = g_system->lockScreen();
		dst->copyRectToSurface(resData, 0, 0, rect);
		g_system->unlockScreen();
		g_system->updateScreen();
	}
}
#endif

Common::Error DgdsEngine::run() {
	initGraphics(320, 200);

	soundData = 0;
	musicData = 0;

	_midiPlayer = new DgdsMidiPlayer();
	assert(_midiPlayer);

	memset(palette, 0, 256*3);
	memset(blacks, 0, 256*3);

	_bmpData.create(320, 200, Graphics::PixelFormat::createFormatCLUT8());

	bottomBuffer.create(320, 200, Graphics::PixelFormat::createFormatCLUT8());
	topBuffer.create(320, 200, Graphics::PixelFormat::createFormatCLUT8());
	resData.create(320, 200, Graphics::PixelFormat::createFormatCLUT8());

	// Rise of the Dragon.
	debug("DgdsEngine::init");

	if (ConfMan.getBool("dump_scripts")) {
		explode(_platform, _rmfName, 0, -1);
		return Common::kNoError;
	}

	g_system->fillScreen(0);

	//playMusic("DYNAMIX.SNG");
	// grayscale palette.
/*
	for (uint i=0; i<256; i++) {
	    palette[i*3+0] = 255-i;
	    palette[i*3+1] = 255-i;
	    palette[i*3+2] = 255-i;
	}
*/
/*
	// Amiga grayscale palette.
	for (uint i=0; i<32; i++) {
	    palette[i*3+0] = 255-i*8;
	    palette[i*3+1] = 255-i*8;
	    palette[i*3+2] = 255-i*8;
	}
	g_system->getPaletteManager()->setPalette(palette, 0, 256);
*/
	Common::EventManager *eventMan = g_system->getEventManager();
	Common::Event ev;

//	browseInit(_platform, _rmfName, this);
	ADSInterpreter interpADS(this);
	TTMInterpreter interpTTM(this);

#if 1
//	TTMData title1Data, title2Data;
	ADSData introData;/*
	interpTTM.load("TITLE1.TTM", &title1Data);
	interpTTM.load("TITLE2.TTM", &title2Data);*/
	interpADS.load("INTRO.ADS", &introData);

//	TTMState title1State, title2State;
	ADSState introState;/*
	interpTTM.init(&title1State, &title1Data);
	interpTTM.init(&title2State, &title2Data);*/
	interpADS.init(&introState, &introData);

	explode(_platform, _rmfName, "DRAGON.FNT", 0);
	explode(_platform, _rmfName, "S55.SDS", 0);
#else
	ADSData introData;
	interpADS.load("TITLE.ADS", &introData);
	ADSState introState;
	interpADS.init(&introState, &introData);

	explode(_platform, _rmfName, "CHINA.FNT", 0);
#endif

	while (!shouldQuit()) {/*
		uint w, h;
		byte *vgaData_;
		byte *binData_;*/

		if (eventMan->pollEvent(ev)) {
			if (ev.type == Common::EVENT_KEYDOWN) {
				switch (ev.kbd.keycode) {
				/*
					case Common::KEYCODE_TAB:	sid++;					break;
					case Common::KEYCODE_UP:	if (id > 0) id--; bk=0;			break;
					case Common::KEYCODE_DOWN:	if (id < BMPs.size()) id++; bk=0;	break;
					case Common::KEYCODE_LEFT:	if (bk > 0) bk--;			break;
					case Common::KEYCODE_RIGHT:	if (bk < (_tcount-1)) bk++;		break;
					*/
					case Common::KEYCODE_ESCAPE:	return Common::kNoError;
					default:			break;
				}
			}
		}

//		browse(_platform, _rmfName, this);

#if 1
/*		if (!interpTTM.run(&title1State))
			if (!interpTTM.run(&title2State))*/
			if (!interpADS.run(&introState)) return Common::kNoError;
#else
		if (!interpADS.run(&introState)) return Common::kNoError;
#endif

//		explode(_platform, _rmfName, "4X5.FNT", 0);
//		explode(_platform, _rmfName, "P6X6.FNT", 0);
//		explode(_platform, _rmfName, "DRAGON.FNT", 0);

//		Common::String txt("WTHE QUICK BROWN BOX JUMPED OVER THE LAZY DOG");
//		Common::String txt("Have a nice trip!");
//		Common::String txt("WAW A*A@A WAW");
/*
		Graphics::Surface *dst;
		dst = g_system->lockScreen();
		_fntP->drawString(dst, txt, 20, 20, 320-20, 1);
		g_system->unlockScreen();
		g_system->updateScreen();
*/
/*
		// BMP:INF|BIN|VGA|MTX browser.
		uint cx, cy;
		cx = cy = 0;
		g_system->fillScreen(0);
		for (uint y=0; cx<_mw; y++) {
			if ((k+y) >= _mw)
			    break;

			for (uint x=0; x<_mh; x++) {
				uint j = (k+y)*_mh+x;
				w = _tw[_mtx[j]]; h = _th[_mtx[j]];
				vgaData_ = vgaData + (_toffsets[_mtx[j]]>>1);
				binData_ = binData + (_toffsets[_mtx[j]]>>1);

				for (uint i=0; i<w*h; i+=2) {
					imgData[i+0]  = ((vgaData_[i/2] & 0xF0)     );
					imgData[i+0] |= ((binData_[i/2] & 0xF0) >> 4);
					imgData[i+1]  = ((vgaData_[i/2] & 0x0F) << 4);
					imgData[i+1] |= ((binData_[i/2] & 0x0F)     );
				}
				g_system->copyRectToScreen(imgData, w, cx, cy, w, h);
				cy += h;
			}
			cy = 0;
			cx += w;
		}
*/
		g_system->delayMillis(40);
	}
	return Common::kNoError;
}

} // End of namespace Dgds


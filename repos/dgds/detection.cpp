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

#include "base/plugins.h"

#include "engines/advancedDetector.h"

#include "dgds/dgds.h"

static const PlainGameDescriptor dgdsGames[] = {
	{"dgds", "Dynamix DGDS game"},
	{"rise", "Rise of the Dragon"},
	{"china", "Heart of China"},

	{0, 0}
};

#include "dgds/detection_tables.h"

namespace Dgds {

class DgdsMetaEngine : public AdvancedMetaEngine {
public:
	DgdsMetaEngine() :
	AdvancedMetaEngine(Dgds::gameDescriptions,
	sizeof(Dgds::DgdsGameDescription), dgdsGames) {

		_singleId = "dgds";
		_guiOptions = GUIO1(GUIO_NONE);
	}

	virtual const char *getName() const {
		return "DGDS";
	}

	virtual const char *getOriginalCopyright() const {
		return "Dynamix Game Development System (C) Dynamix";
	}

	virtual bool createInstance(OSystem *syst, Engine **engine, const ADGameDescription *desc) const;
};

bool DgdsMetaEngine::createInstance(OSystem *syst, Engine **engine, const ADGameDescription *desc) const {
	const Dgds::DgdsGameDescription *gd = (const Dgds::DgdsGameDescription *)desc;
	if (gd) {
		*engine = new Dgds::DgdsEngine(syst, gd);
	}
	return gd != 0;
}

} // End of namespace Dgds

#if PLUGIN_ENABLED_DYNAMIC(DGDS)
	REGISTER_PLUGIN_DYNAMIC(DGDS, PLUGIN_TYPE_ENGINE, Dgds::DgdsMetaEngine);
#else
	REGISTER_PLUGIN_STATIC(DGDS, PLUGIN_TYPE_ENGINE, Dgds::DgdsMetaEngine);
#endif
